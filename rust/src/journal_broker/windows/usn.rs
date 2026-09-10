use std::collections::{BTreeMap, BTreeSet};
use std::ffi::c_void;
use std::io;
use std::mem::size_of;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ACCESS_DENIED, ERROR_HANDLE_EOF, ERROR_INVALID_FUNCTION,
    ERROR_IO_INCOMPLETE, ERROR_IO_PENDING, ERROR_JOURNAL_DELETE_IN_PROGRESS,
    ERROR_JOURNAL_ENTRY_DELETED, ERROR_JOURNAL_NOT_ACTIVE, ERROR_NOT_SUPPORTED,
    ERROR_OPERATION_ABORTED, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Storage::FileSystem::{
    ExtendedFileIdType, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_REPARSE_POINT,
    FILE_ATTRIBUTE_TAG_INFO, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_ID_128,
    FILE_ID_DESCRIPTOR, FILE_ID_DESCRIPTOR_0, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, FileAttributeTagInfo, FileIdType,
    GetFileInformationByHandleEx, GetFinalPathNameByHandleW, OpenFileById, VOLUME_NAME_GUID,
};
use windows_sys::Win32::System::IO::{
    CancelIoEx, DeviceIoControl, GetOverlappedResult, OVERLAPPED,
};
use windows_sys::Win32::System::Ioctl::{
    FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V1, USN_JOURNAL_DATA_V2,
    USN_REASON_RENAME_NEW_NAME, USN_REASON_RENAME_OLD_NAME,
};
use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

use crate::journal_broker::service::{
    BackendError, BackendJournalCandidate, BackendJournalHandoff, BackendPendingRename,
    BoundedJournalPageBuilder, ExistingJournalMetadata, JournalRange, JournalReadLimits,
    JournalReadProof, PinnedRootState, RequestContext, SharedBackendJournalPage,
};
use crate::journal_broker::wire::{
    CandidateKind, RootRelativeScope, normalize_backend_path, scope_under_root,
};
use crate::windows_usn::{
    FileReference, ParsedUsnRecord, ReferenceHistories, ReferenceResolutionError, UsnParseError,
    parse_journal_buffer as parse_shared_journal_buffer, reference_histories,
    resolve_reference_path_with,
};

const JOURNAL_BUFFER_BYTES: usize = 256 * 1024;
const MAX_RAW_RECORDS_PER_PAGE: usize = 16 * 1024;
const MAX_PATH_UTF16_UNITS: usize = 32_767;
const MAX_SINGLE_RAW_RECORD_EVIDENCE_BYTES: usize =
    size_of::<ParsedUsnRecord>() + MAX_PATH_UTF16_UNITS * 2;
const MAX_RAW_PAGE_RETAINED_BYTES: usize =
    JOURNAL_BUFFER_BYTES + MAX_RAW_RECORDS_PER_PAGE * size_of::<ParsedUsnRecord>();
const DEVICE_IO_POLL_INTERVAL: Duration = Duration::from_millis(25);
const DEVICE_IO_CANCEL_DRAIN_TIMEOUT: Duration = Duration::from_secs(1);

pub(super) struct NativePendingRenameCarry {
    pub(super) root: Option<std::sync::Arc<PinnedRootState>>,
    pub(super) read_root_index: Option<usize>,
    pub(super) file_reference: Vec<u8>,
    pub(super) old_usn: i64,
    pub(super) previous_relative_path: String,
    pub(super) is_directory: bool,
}

pub(super) fn query_existing_journal(
    root: &PinnedRootState,
    context: &mut RequestContext<'_>,
) -> Result<ExistingJournalMetadata, BackendError> {
    require_live_root(root)?;
    let metadata = query_journal(
        root.volume_handle()
            .map_err(|_| BackendError::RootUnavailable)?,
        context,
    )?;
    require_live_root(root)?;
    Ok(metadata)
}

pub(super) fn read_existing_journal_page(
    root: &PinnedRootState,
    journal: ExistingJournalMetadata,
    range: JournalRange,
    page: &mut BoundedJournalPageBuilder,
    context: &mut RequestContext<'_>,
) -> Result<JournalReadProof, BackendError> {
    require_live_root(root)?;
    let (max_records, max_evidence_bytes) = page.limits();
    let volume_handle = root
        .volume_handle()
        .map_err(|_| BackendError::RootUnavailable)?;
    let raw = read_raw_journal_page(
        volume_handle,
        journal,
        range,
        max_records,
        max_evidence_bytes,
        context,
    )?;
    let resolved =
        require_live_root(root).and_then(|()| resolve_records(root, &raw.records, context))?;
    let candidates = root_scoped_candidates(root, resolved)
        .and_then(|candidates| require_live_root(root).map(|()| candidates))?;
    if candidates.len() > max_records {
        return Err(BackendError::EvidenceLimitExceeded);
    }
    for candidate in candidates {
        page.push(candidate)?;
    }
    Ok(raw.proof)
}

pub(super) fn read_existing_journal_volume_page(
    roots: &[std::sync::Arc<PinnedRootState>],
    root_start_usns: &[i64],
    pending_renames: &[NativePendingRenameCarry],
    journal: ExistingJournalMetadata,
    range: JournalRange,
    limits: JournalReadLimits,
    context: &mut RequestContext<'_>,
) -> Result<SharedBackendJournalPage, BackendError> {
    if roots.is_empty() || roots.len() != root_start_usns.len() {
        return Err(BackendError::RootUnavailable);
    }
    let mut candidates_by_root = Vec::with_capacity(roots.len());
    let mut volume_handles = Vec::with_capacity(roots.len());
    for root in roots {
        match require_live_root(root).and_then(|()| {
            root.volume_handle()
                .map_err(|_| BackendError::RootUnavailable)
        }) {
            Ok(handle) => {
                candidates_by_root.push(Ok(Vec::new()));
                volume_handles.push(Some(handle));
            }
            Err(error) => {
                candidates_by_root.push(Err(error));
                volume_handles.push(None);
            }
        }
    }
    let Some(volume_handle) = volume_handles.iter().flatten().copied().next() else {
        return Ok(SharedBackendJournalPage {
            candidates_by_root,
            handoffs: Vec::new(),
            pending_renames: Vec::new(),
            rename_barriers_by_root: vec![None; roots.len()],
            proof: None,
        });
    };
    let raw = read_raw_journal_page(
        volume_handle,
        journal,
        range,
        MAX_RAW_RECORDS_PER_PAGE,
        MAX_RAW_PAGE_RETAINED_BYTES,
        context,
    )?;
    let mut rename_barriers_by_root = vec![None; roots.len()];
    let mut uncertainty_barrier = None;
    for (index, candidates) in candidates_by_root.iter().enumerate() {
        if candidates.is_err()
            && let Some(usn) = first_rename_usn_at_or_after(&raw.records, root_start_usns[index])
        {
            rename_barriers_by_root[index] = Some(usn);
            uncertainty_barrier = Some(uncertainty_barrier.map_or(usn, |old: i64| old.min(usn)));
        }
    }
    let mut resolved_records = None;
    for (index, root) in roots.iter().enumerate() {
        if candidates_by_root[index].is_err() {
            continue;
        }
        match require_live_root(root).and_then(|()| resolve_records(root, &raw.records, context)) {
            Ok(resolved) => {
                resolved_records = Some(resolved);
                break;
            }
            Err(error) => {
                candidates_by_root[index] = Err(error);
                if let Some(usn) =
                    first_rename_usn_at_or_after(&raw.records, root_start_usns[index])
                {
                    rename_barriers_by_root[index] = Some(usn);
                    uncertainty_barrier =
                        Some(uncertainty_barrier.map_or(usn, |old: i64| old.min(usn)));
                }
            }
        }
    }
    let Some(records) = resolved_records else {
        return Ok(SharedBackendJournalPage {
            candidates_by_root,
            handoffs: Vec::new(),
            pending_renames: Vec::new(),
            rename_barriers_by_root,
            proof: None,
        });
    };
    for (index, root) in roots.iter().enumerate() {
        if candidates_by_root[index].is_ok()
            && let Err(error) = require_live_root(root)
        {
            candidates_by_root[index] = Err(error);
            if let Some(usn) = first_rename_usn_at_or_after(&raw.records, root_start_usns[index]) {
                rename_barriers_by_root[index] = Some(usn);
                uncertainty_barrier =
                    Some(uncertainty_barrier.map_or(usn, |old: i64| old.min(usn)));
            }
        }
    }
    let mut events = semantic_events(
        roots,
        root_start_usns,
        pending_renames,
        records,
        &mut candidates_by_root,
        &mut uncertainty_barrier,
        &mut rename_barriers_by_root,
    )?;
    events.sort_by_key(|event| event.usn);
    let (max_records, max_evidence_bytes) = limits.parts();
    let (raw_covered_until_usn, _, after) = raw.proof.parts();
    let (covered_until_usn, handoffs, durable_pending) = apply_semantic_limits(
        &mut candidates_by_root,
        events,
        raw_covered_until_usn,
        uncertainty_barrier,
        root_start_usns,
        max_records,
        max_evidence_bytes,
    )?;
    let proof = if candidates_by_root.iter().any(Result::is_ok) {
        Some(JournalReadProof::after_read(
            covered_until_usn,
            covered_until_usn == range.bounds().1,
            after,
        )?)
    } else {
        None
    };
    Ok(SharedBackendJournalPage {
        proof,
        candidates_by_root,
        handoffs,
        pending_renames: durable_pending,
        rename_barriers_by_root,
    })
}

fn first_rename_usn_at_or_after(records: &[ParsedUsnRecord], start_usn: i64) -> Option<i64> {
    records.iter().find_map(|record| {
        (record.usn >= start_usn
            && record.reason & (USN_REASON_RENAME_OLD_NAME | USN_REASON_RENAME_NEW_NAME) != 0)
            .then_some(record.usn)
    })
}

#[derive(Default)]
struct SemanticEvent {
    usn: i64,
    candidates: Vec<(usize, BackendJournalCandidate)>,
    handoff: Option<BackendJournalHandoff>,
    pending: Option<BackendPendingRename>,
}

impl SemanticEvent {
    fn costs(&self) -> Result<(usize, usize), BackendError> {
        let mut records = self.candidates.len();
        let mut evidence = 0_usize;
        for (_, candidate) in &self.candidates {
            evidence = evidence
                .checked_add(candidate.evidence_bytes()?)
                .ok_or(BackendError::EvidenceLimitExceeded)?;
        }
        if let Some(handoff) = &self.handoff {
            records = records
                .checked_add(1)
                .ok_or(BackendError::EvidenceLimitExceeded)?;
            evidence = evidence
                .checked_add(handoff.previous_absolute_path_utf16.len().saturating_mul(2))
                .and_then(|value| {
                    value.checked_add(handoff.current_absolute_path_utf16.len().saturating_mul(2))
                })
                .and_then(|value| value.checked_add(handoff.file_reference.len()))
                .ok_or(BackendError::EvidenceLimitExceeded)?;
        }
        if let Some(pending) = &self.pending {
            records = records
                .checked_add(1)
                .ok_or(BackendError::EvidenceLimitExceeded)?;
            evidence = evidence
                .checked_add(pending.previous_absolute_path_utf16.len().saturating_mul(2))
                .and_then(|value| value.checked_add(pending.file_reference.len()))
                .ok_or(BackendError::EvidenceLimitExceeded)?;
        }
        Ok((records, evidence))
    }

    fn affected_roots(&self) -> BTreeSet<usize> {
        let mut roots = self
            .candidates
            .iter()
            .map(|(index, _)| *index)
            .collect::<BTreeSet<_>>();
        if let Some(handoff) = &self.handoff {
            if let Some(index) = handoff.previous_root_index {
                roots.insert(index);
            }
            roots.insert(handoff.current_root_index);
        }
        if let Some(pending) = &self.pending {
            roots.insert(pending.root_index);
        }
        roots
    }
}

struct SemanticRootContext<'a> {
    volume_ids: &'a [&'a str],
    roots: &'a [std::sync::Arc<PinnedRootState>],
    root_start_usns: &'a [i64],
}

struct RenamePairEvent {
    previous_path: Vec<u16>,
    previous_usn: i64,
    previous_roots: Vec<usize>,
    previous_pending_index: Option<usize>,
    current: ResolvedRecord,
}

fn semantic_events(
    roots: &[std::sync::Arc<PinnedRootState>],
    root_start_usns: &[i64],
    pending_renames: &[NativePendingRenameCarry],
    records: Vec<ResolvedRecord>,
    candidates_by_root: &mut [Result<Vec<BackendJournalCandidate>, BackendError>],
    uncertainty_barrier: &mut Option<i64>,
    rename_barriers_by_root: &mut [Option<i64>],
) -> Result<Vec<SemanticEvent>, BackendError> {
    let volume_ids = roots
        .iter()
        .map(|root| root.volume_id())
        .collect::<Vec<_>>();
    let root_context = SemanticRootContext {
        volume_ids: &volume_ids,
        roots,
        root_start_usns,
    };
    let mut events = Vec::new();
    let mut old_names = BTreeMap::<FileReference, ResolvedRecord>::new();
    let mut pending_by_reference = BTreeMap::<FileReference, Vec<usize>>::new();
    for (index, pending) in pending_renames.iter().enumerate() {
        let reference = file_reference_from_bytes(&pending.file_reference)?;
        pending_by_reference
            .entry(reference)
            .or_default()
            .push(index);
    }
    for current in records {
        if current.reason & USN_REASON_RENAME_OLD_NAME != 0 {
            old_names.insert(current.file_reference, current);
            continue;
        }
        if current.reason & USN_REASON_RENAME_NEW_NAME != 0 {
            if let Some(previous) = old_names.remove(&current.file_reference) {
                let (previous_roots, _) = roots_for_rename_path(
                    roots,
                    root_start_usns,
                    candidates_by_root,
                    &previous.path,
                    previous.usn,
                    uncertainty_barrier,
                    rename_barriers_by_root,
                );
                push_pair_event(
                    &root_context,
                    candidates_by_root,
                    &mut events,
                    uncertainty_barrier,
                    rename_barriers_by_root,
                    RenamePairEvent {
                        previous_path: previous.path,
                        previous_usn: previous.usn,
                        previous_roots,
                        previous_pending_index: None,
                        current,
                    },
                )?;
                continue;
            }
            let pending_index =
                pending_by_reference
                    .get(&current.file_reference)
                    .and_then(|indices| {
                        indices
                            .iter()
                            .copied()
                            .filter(|index| {
                                pending_renames[*index].old_usn < current.usn
                                    && pending_renames[*index].is_directory == current.is_directory
                            })
                            .max_by_key(|index| pending_renames[*index].old_usn)
                    });
            if let Some(pending_index) = pending_index {
                let pending = &pending_renames[pending_index];
                let (current_roots, current_uncertain) = roots_for_rename_path(
                    roots,
                    root_start_usns,
                    candidates_by_root,
                    &current.path,
                    current.usn,
                    uncertainty_barrier,
                    rename_barriers_by_root,
                );
                let Some(previous_root) = pending.root.as_ref() else {
                    *uncertainty_barrier =
                        Some(uncertainty_barrier.map_or(current.usn, |old| old.min(current.usn)));
                    continue;
                };
                let previous_path = join_root_relative(
                    previous_root.canonical_root_utf16(),
                    &pending.previous_relative_path,
                )?;
                push_pair_event_with_current_roots(
                    root_context.volume_ids,
                    candidates_by_root,
                    &mut events,
                    RenamePairEvent {
                        previous_path,
                        previous_usn: pending.old_usn,
                        previous_roots: pending.read_root_index.into_iter().collect(),
                        previous_pending_index: Some(pending_index),
                        current,
                    },
                    current_roots,
                    current_uncertain,
                )?;
                continue;
            }
        }
        let current_roots = roots_for_path(
            roots,
            root_start_usns,
            candidates_by_root,
            &current.path,
            current.usn,
        );
        let mut event = SemanticEvent {
            usn: current.usn,
            ..SemanticEvent::default()
        };
        for index in current_roots {
            event.candidates.push((
                index,
                scoped_candidate(roots[index].volume_id(), &current.path, &current)?,
            ));
        }
        if !event.candidates.is_empty() {
            events.push(event);
        }
    }
    for (_, previous) in old_names {
        let (previous_roots, _) = roots_for_rename_path(
            roots,
            root_start_usns,
            candidates_by_root,
            &previous.path,
            previous.usn,
            uncertainty_barrier,
            rename_barriers_by_root,
        );
        if let [root_index] = previous_roots.as_slice() {
            events.push(SemanticEvent {
                usn: previous.usn,
                pending: Some(BackendPendingRename {
                    root_index: *root_index,
                    previous_absolute_path_utf16: previous.path,
                    file_reference: previous.file_reference.bytes(),
                    old_usn: previous.usn,
                    is_directory: previous.is_directory,
                }),
                ..SemanticEvent::default()
            });
        } else if previous_roots.len() > 1 {
            for index in previous_roots {
                candidates_by_root[index] = Err(BackendError::RootUnavailable);
            }
        }
    }
    Ok(events)
}

fn push_pair_event(
    root_context: &SemanticRootContext<'_>,
    candidates_by_root: &mut [Result<Vec<BackendJournalCandidate>, BackendError>],
    events: &mut Vec<SemanticEvent>,
    uncertainty_barrier: &mut Option<i64>,
    rename_barriers_by_root: &mut [Option<i64>],
    pair: RenamePairEvent,
) -> Result<(), BackendError> {
    let (current_roots, current_uncertain) = roots_for_rename_path(
        root_context.roots,
        root_context.root_start_usns,
        candidates_by_root,
        &pair.current.path,
        pair.current.usn,
        uncertainty_barrier,
        rename_barriers_by_root,
    );
    push_pair_event_with_current_roots(
        root_context.volume_ids,
        candidates_by_root,
        events,
        pair,
        current_roots,
        current_uncertain,
    )
}

fn push_pair_event_with_current_roots(
    volume_ids: &[&str],
    candidates_by_root: &mut [Result<Vec<BackendJournalCandidate>, BackendError>],
    events: &mut Vec<SemanticEvent>,
    pair: RenamePairEvent,
    current_roots: Vec<usize>,
    current_uncertain: bool,
) -> Result<(), BackendError> {
    let RenamePairEvent {
        previous_path,
        previous_usn,
        previous_roots,
        previous_pending_index,
        current,
    } = pair;
    if current_uncertain && previous_pending_index.is_none() {
        if let [root_index] = previous_roots.as_slice() {
            events.push(SemanticEvent {
                usn: previous_usn,
                pending: Some(BackendPendingRename {
                    root_index: *root_index,
                    previous_absolute_path_utf16: previous_path,
                    file_reference: current.file_reference.bytes(),
                    old_usn: previous_usn,
                    is_directory: current.is_directory,
                }),
                ..SemanticEvent::default()
            });
        }
        return Ok(());
    }
    let same_root = matches!(
        (previous_roots.as_slice(), current_roots.as_slice()),
        ([previous], [current_index]) if previous == current_index
    );
    let mut event = SemanticEvent {
        usn: current.usn,
        ..SemanticEvent::default()
    };
    if same_root {
        let root_index = previous_roots[0];
        event.candidates.push((
            root_index,
            BackendJournalCandidate::copy_from_bounded(
                volume_ids[root_index],
                &current.path,
                Some(&previous_path),
                &current.file_reference.bytes(),
                current.usn,
                CandidateKind::Rename,
                current.is_directory,
            )?,
        ));
    } else {
        for index in &previous_roots {
            if candidates_by_root[*index].is_ok() {
                event.candidates.push((
                    *index,
                    scoped_candidate_from_parts(
                        volume_ids[*index],
                        &previous_path,
                        current.file_reference,
                        current.usn,
                        current.is_directory,
                    )?,
                ));
            }
        }
        for index in &current_roots {
            if candidates_by_root[*index].is_ok() {
                event.candidates.push((
                    *index,
                    scoped_candidate(volume_ids[*index], &current.path, &current)?,
                ));
            }
        }
        if let [current_root_index] = current_roots.as_slice() {
            let previous_root_index = previous_roots.as_slice().first().copied();
            let is_cross_root = previous_pending_index.is_some()
                || previous_root_index.is_some_and(|index| index != *current_root_index);
            if is_cross_root && previous_roots.len() <= 1 {
                event.handoff = Some(BackendJournalHandoff {
                    previous_root_index: previous_pending_index
                        .is_none()
                        .then_some(previous_root_index)
                        .flatten(),
                    previous_pending_index,
                    current_root_index: *current_root_index,
                    previous_absolute_path_utf16: previous_path,
                    current_absolute_path_utf16: current.path.clone(),
                    file_reference: current.file_reference.bytes(),
                    old_usn: previous_usn,
                    usn: current.usn,
                    is_directory: current.is_directory,
                });
            }
        }
    }
    if !event.candidates.is_empty() || event.handoff.is_some() {
        events.push(event);
    }
    Ok(())
}

fn roots_for_path(
    roots: &[std::sync::Arc<PinnedRootState>],
    root_start_usns: &[i64],
    candidates_by_root: &mut [Result<Vec<BackendJournalCandidate>, BackendError>],
    path: &[u16],
    usn: i64,
) -> Vec<usize> {
    roots_for_path_with(
        roots.len(),
        root_start_usns,
        candidates_by_root,
        path,
        usn,
        |index, path| path_is_inside(&roots[index], path),
    )
}

fn roots_for_rename_path(
    roots: &[std::sync::Arc<PinnedRootState>],
    root_start_usns: &[i64],
    candidates_by_root: &mut [Result<Vec<BackendJournalCandidate>, BackendError>],
    path: &[u16],
    usn: i64,
    uncertainty_barrier: &mut Option<i64>,
    rename_barriers_by_root: &mut [Option<i64>],
) -> (Vec<usize>, bool) {
    roots_for_rename_path_with(
        roots.len(),
        root_start_usns,
        candidates_by_root,
        path,
        usn,
        uncertainty_barrier,
        rename_barriers_by_root,
        |index, path| path_is_inside(&roots[index], path),
    )
}

#[allow(clippy::too_many_arguments)]
fn roots_for_rename_path_with(
    root_count: usize,
    root_start_usns: &[i64],
    candidates_by_root: &mut [Result<Vec<BackendJournalCandidate>, BackendError>],
    path: &[u16],
    usn: i64,
    uncertainty_barrier: &mut Option<i64>,
    rename_barriers_by_root: &mut [Option<i64>],
    is_inside: impl FnMut(usize, &[u16]) -> Result<bool, BackendError>,
) -> (Vec<usize>, bool) {
    let before = candidates_by_root
        .iter()
        .map(Result::is_ok)
        .collect::<Vec<_>>();
    let owners = roots_for_path_with(
        root_count,
        root_start_usns,
        candidates_by_root,
        path,
        usn,
        is_inside,
    );
    for index in &owners {
        rename_barriers_by_root[*index] =
            Some(rename_barriers_by_root[*index].map_or(usn, |old| old.min(usn)));
    }
    let mut uncertain = false;
    for (index, was_healthy) in before.into_iter().enumerate() {
        if was_healthy && candidates_by_root[index].is_err() {
            uncertain = true;
            rename_barriers_by_root[index] =
                Some(rename_barriers_by_root[index].map_or(usn, |old| old.min(usn)));
            *uncertainty_barrier = Some(uncertainty_barrier.map_or(usn, |old| old.min(usn)));
        }
    }
    (owners, uncertain)
}

fn roots_for_path_with(
    root_count: usize,
    root_start_usns: &[i64],
    candidates_by_root: &mut [Result<Vec<BackendJournalCandidate>, BackendError>],
    path: &[u16],
    usn: i64,
    mut is_inside: impl FnMut(usize, &[u16]) -> Result<bool, BackendError>,
) -> Vec<usize> {
    let mut inside = Vec::new();
    for index in 0..root_count {
        if candidates_by_root[index].is_err() || usn < root_start_usns[index] {
            continue;
        }
        match is_inside(index, path) {
            Ok(true) => inside.push(index),
            Ok(false) => {}
            Err(error) => candidates_by_root[index] = Err(error),
        }
    }
    inside
}

fn scoped_candidate(
    volume_id: &str,
    path: &[u16],
    record: &ResolvedRecord,
) -> Result<BackendJournalCandidate, BackendError> {
    scoped_candidate_from_parts(
        volume_id,
        path,
        record.file_reference,
        record.usn,
        record.is_directory,
    )
}

fn scoped_candidate_from_parts(
    volume_id: &str,
    path: &[u16],
    file_reference: FileReference,
    usn: i64,
    is_directory: bool,
) -> Result<BackendJournalCandidate, BackendError> {
    BackendJournalCandidate::copy_from_bounded(
        volume_id,
        path,
        None,
        &file_reference.bytes(),
        usn,
        if is_directory {
            CandidateKind::Subtree
        } else {
            CandidateKind::Path
        },
        is_directory,
    )
}

fn apply_semantic_limits(
    candidates_by_root: &mut [Result<Vec<BackendJournalCandidate>, BackendError>],
    events: Vec<SemanticEvent>,
    raw_covered_until_usn: i64,
    uncertainty_barrier: Option<i64>,
    root_start_usns: &[i64],
    max_records: usize,
    max_evidence_bytes: usize,
) -> Result<(i64, Vec<BackendJournalHandoff>, Vec<BackendPendingRename>), BackendError> {
    let mut total_records = 0_usize;
    let mut total_evidence = 0_usize;
    let mut covered_until_usn = uncertainty_barrier.map_or(raw_covered_until_usn, |barrier| {
        raw_covered_until_usn.min(barrier)
    });
    let mut handoffs = Vec::new();
    let mut pending_renames = Vec::new();
    for mut event in events {
        if uncertainty_barrier.is_some_and(|barrier| event.usn >= barrier) {
            break;
        }
        event
            .candidates
            .retain(|(index, _)| candidates_by_root[*index].is_ok());
        if event.handoff.as_ref().is_some_and(|handoff| {
            candidates_by_root[handoff.current_root_index].is_err()
                || handoff
                    .previous_root_index
                    .is_some_and(|index| candidates_by_root[index].is_err())
        }) {
            event.handoff = None;
        }
        if event
            .pending
            .as_ref()
            .is_some_and(|pending| candidates_by_root[pending.root_index].is_err())
        {
            event.pending = None;
        }
        if event.candidates.is_empty() && event.handoff.is_none() && event.pending.is_none() {
            continue;
        }
        let (records, evidence) = event.costs()?;
        let next_records = total_records.checked_add(records);
        let next_evidence = total_evidence.checked_add(evidence);
        if next_records.is_none_or(|value| value > max_records)
            || next_evidence.is_none_or(|value| value > max_evidence_bytes)
        {
            let minimum_start = root_start_usns
                .iter()
                .enumerate()
                .filter(|(index, _)| candidates_by_root[*index].is_ok())
                .map(|(_, start)| *start)
                .min()
                .ok_or(BackendError::RootUnavailable)?;
            if total_records == 0 && event.usn <= minimum_start {
                for index in event.affected_roots() {
                    candidates_by_root[index] = Err(BackendError::EvidenceLimitExceeded);
                }
                continue;
            }
            covered_until_usn = event.usn.min(raw_covered_until_usn);
            break;
        }
        total_records = next_records.expect("bounded semantic record count");
        total_evidence = next_evidence.expect("bounded semantic evidence count");
        for (index, candidate) in event.candidates {
            if let Ok(candidates) = &mut candidates_by_root[index] {
                candidates.push(candidate);
            }
        }
        if let Some(handoff) = event.handoff {
            handoffs.push(handoff);
        }
        if let Some(pending) = event.pending {
            pending_renames.push(pending);
        }
    }
    Ok((covered_until_usn, handoffs, pending_renames))
}

fn file_reference_from_bytes(bytes: &[u8]) -> Result<FileReference, BackendError> {
    match bytes {
        [a, b, c, d, e, f, g, h] => Ok(FileReference::V2([*a, *b, *c, *d, *e, *f, *g, *h])),
        [a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p] => Ok(FileReference::V3([
            *a, *b, *c, *d, *e, *f, *g, *h, *i, *j, *k, *l, *m, *n, *o, *p,
        ])),
        _ => Err(BackendError::RecordUnsupported),
    }
}

fn join_root_relative(root: &[u16], relative: &str) -> Result<Vec<u16>, BackendError> {
    let mut path = root.to_vec();
    if !path.ends_with(&[b'\\' as u16]) {
        path.push(b'\\' as u16);
    }
    path.extend(relative.encode_utf16());
    if path.len() > MAX_PATH_UTF16_UNITS {
        return Err(BackendError::RecordUnsupported);
    }
    normalize_backend_path(&path).map_err(|_| BackendError::RecordUnsupported)?;
    Ok(path)
}

struct RawJournalPage {
    records: Vec<ParsedUsnRecord>,
    proof: JournalReadProof,
}

fn read_raw_journal_page(
    volume_handle: HANDLE,
    journal: ExistingJournalMetadata,
    range: JournalRange,
    max_records: usize,
    max_evidence_bytes: usize,
    context: &mut RequestContext<'_>,
) -> Result<RawJournalPage, BackendError> {
    let (start_usn, end_usn) = range.bounds();
    context.bounded_step()?;
    let (raw_records, covered_until_usn) = read_single_native_journal_buffer_with(
        start_usn,
        end_usn,
        max_records,
        max_evidence_bytes,
        |physical_start_usn| {
            let request = READ_USN_JOURNAL_DATA_V1 {
                StartUsn: physical_start_usn,
                ReasonMask: u32::MAX,
                ReturnOnlyOnClose: 0,
                Timeout: 0,
                BytesToWaitFor: 0,
                UsnJournalID: journal_id(journal),
                MinMajorVersion: 2,
                MaxMajorVersion: 3,
            };
            let mut output = vec![0_u8; JOURNAL_BUFFER_BYTES];
            let bytes = device_io_control(
                volume_handle,
                FSCTL_READ_USN_JOURNAL,
                (&raw const request).cast(),
                size_of::<READ_USN_JOURNAL_DATA_V1>(),
                output.as_mut_ptr().cast(),
                output.len(),
                context,
            )?;
            output.truncate(bytes);
            Ok(output)
        },
    )?;

    context.bounded_step()?;
    let after = query_journal(volume_handle, context)?;
    Ok(RawJournalPage {
        records: raw_records,
        proof: JournalReadProof::after_read(
            covered_until_usn,
            covered_until_usn == end_usn,
            after,
        )?,
    })
}

fn read_single_native_journal_buffer_with(
    start_usn: i64,
    end_usn: i64,
    max_records: usize,
    max_evidence_bytes: usize,
    physical_read: impl FnOnce(i64) -> Result<Vec<u8>, BackendError>,
) -> Result<(Vec<ParsedUsnRecord>, i64), BackendError> {
    let output = physical_read(start_usn)?;
    let (next_usn, batch) = parse_shared_journal_buffer(&output).map_err(map_parse_error)?;
    let mut raw_records = Vec::new();
    let mut retained_bytes = 0_usize;
    let first_unretained_usn = retain_bounded_raw_records(
        &mut raw_records,
        &mut retained_bytes,
        batch,
        end_usn,
        max_records.min(MAX_RAW_RECORDS_PER_PAGE),
        max_evidence_bytes.max(MAX_SINGLE_RAW_RECORD_EVIDENCE_BYTES),
    )?;
    let covered_until_usn = first_unretained_usn.unwrap_or(next_usn).min(end_usn);
    if covered_until_usn <= start_usn {
        return Err(BackendError::JournalDiscontinuous);
    }
    Ok((raw_records, covered_until_usn))
}

fn retain_bounded_raw_records(
    retained: &mut Vec<ParsedUsnRecord>,
    retained_bytes: &mut usize,
    records: Vec<ParsedUsnRecord>,
    end_usn: i64,
    max_records: usize,
    max_evidence_bytes: usize,
) -> Result<Option<i64>, BackendError> {
    for record in records.into_iter().filter(|record| record.usn < end_usn) {
        let record_bytes = size_of::<ParsedUsnRecord>()
            .checked_add(
                record
                    .name
                    .encode_utf16()
                    .count()
                    .checked_mul(2)
                    .ok_or(BackendError::EvidenceLimitExceeded)?,
            )
            .ok_or(BackendError::EvidenceLimitExceeded)?;
        let next_bytes = retained_bytes
            .checked_add(record_bytes)
            .ok_or(BackendError::EvidenceLimitExceeded)?;
        if retained.len() >= max_records || next_bytes > max_evidence_bytes {
            return Ok(Some(record.usn));
        }
        *retained_bytes = next_bytes;
        retained.push(record);
    }
    Ok(None)
}

fn require_live_root(root: &PinnedRootState) -> Result<(), BackendError> {
    root.revalidate_identity()
        .map_err(|_| BackendError::JournalDiscontinuous)
}

fn journal_id(metadata: ExistingJournalMetadata) -> u64 {
    // ExistingJournalMetadata is already validated at construction. Keep field access inside the
    // service-owned boundary through its lossless debug-free byte representation helper below.
    metadata_parts(metadata).0
}

fn metadata_parts(metadata: ExistingJournalMetadata) -> (u64, i64, i64) {
    // This helper is replaced by crate-visible accessors rather than any layout cast.
    metadata.parts()
}

fn query_journal(
    handle: HANDLE,
    context: &mut RequestContext<'_>,
) -> Result<ExistingJournalMetadata, BackendError> {
    let mut output = USN_JOURNAL_DATA_V2::default();
    let bytes = device_io_control(
        handle,
        FSCTL_QUERY_USN_JOURNAL,
        null(),
        0,
        (&raw mut output).cast(),
        size_of::<USN_JOURNAL_DATA_V2>(),
        context,
    )?;
    if bytes < 56
        || output.FirstUsn < 0
        || output.NextUsn < output.FirstUsn
        || output.MinSupportedMajorVersion > 3
        || output.MaxSupportedMajorVersion < 2
    {
        return Err(BackendError::JournalDiscontinuous);
    }
    ExistingJournalMetadata::from_query(output.UsnJournalID, output.FirstUsn, output.NextUsn)
}

fn device_io_control(
    handle: HANDLE,
    code: u32,
    input: *const c_void,
    input_bytes: usize,
    output: *mut c_void,
    output_bytes: usize,
    context: &mut RequestContext<'_>,
) -> Result<usize, BackendError> {
    let output_capacity = output_bytes;
    let input_bytes =
        u32::try_from(input_bytes).map_err(|_| BackendError::EvidenceLimitExceeded)?;
    let output_bytes =
        u32::try_from(output_bytes).map_err(|_| BackendError::EvidenceLimitExceeded)?;
    // SAFETY: null security attributes and name create a private manual-reset event owned below.
    let event = OwnedHandle::new(unsafe { CreateEventW(null(), 1, 0, null()) })?;
    let mut overlapped = Box::<OVERLAPPED>::default();
    overlapped.hEvent = event.raw();
    // SAFETY: the admitted broker exposes only QUERY/READ control codes. The volume was opened
    // overlapped; input/output buffers, event, and stable boxed OVERLAPPED remain live until
    // GetOverlappedResult observes completion, including after CancelIoEx.
    let succeeded = unsafe {
        DeviceIoControl(
            handle,
            code,
            input,
            input_bytes,
            output,
            output_bytes,
            null_mut(),
            overlapped.as_mut(),
        )
    };
    if succeeded == 0 {
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) {
            return Err(map_device_error(error));
        }
    }
    let returned = loop {
        if let Some(returned) = poll_device_io(handle, overlapped.as_ref())? {
            break returned;
        }
        let wait = match context.bounded_step() {
            Ok(remaining) => remaining.min(DEVICE_IO_POLL_INTERVAL),
            Err(reason) => {
                cancel_device_io_and_drain(handle, overlapped.as_ref(), event.raw());
                return Err(reason);
            }
        };
        // SAFETY: event is a live waitable handle and the wait is bounded by the request step.
        match unsafe { WaitForSingleObject(event.raw(), duration_millis(wait)) } {
            WAIT_OBJECT_0 | WAIT_TIMEOUT => {}
            _ => {
                cancel_device_io_and_drain(handle, overlapped.as_ref(), event.raw());
                return Err(BackendError::Unavailable);
            }
        }
    };
    if returned > output_capacity {
        return Err(BackendError::RecordUnsupported);
    }
    Ok(returned)
}

fn poll_device_io(handle: HANDLE, overlapped: &OVERLAPPED) -> Result<Option<usize>, BackendError> {
    let mut returned = 0_u32;
    // SAFETY: handle and the stable OVERLAPPED remain live. FALSE makes this a nonblocking query.
    if unsafe { GetOverlappedResult(handle, overlapped, &mut returned, 0) } != 0 {
        return Ok(Some(returned as usize));
    }
    let error = io::Error::last_os_error();
    match error.raw_os_error().map(|value| value as u32) {
        Some(ERROR_IO_INCOMPLETE) => Ok(None),
        Some(ERROR_OPERATION_ABORTED) => Err(BackendError::Cancelled),
        _ => Err(map_device_error(error)),
    }
}

fn cancel_device_io_and_drain(handle: HANDLE, overlapped: &OVERLAPPED, event: HANDLE) {
    // SAFETY: this exact OVERLAPPED belongs to handle and stays live until this function either
    // observes terminal completion or terminates the broker process at the hard deadline.
    let _ = unsafe { CancelIoEx(handle, overlapped) };
    let deadline = Instant::now() + DEVICE_IO_CANCEL_DRAIN_TIMEOUT;
    loop {
        let mut returned = 0_u32;
        // SAFETY: all operation storage remains live; any terminal status proves kernel ownership
        // of the OVERLAPPED and buffers has ended.
        if unsafe { GetOverlappedResult(handle, overlapped, &mut returned, 0) } != 0 {
            return;
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(ERROR_IO_INCOMPLETE as i32) {
            return;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            std::process::abort();
        }
        // SAFETY: event stays live and each wait is bounded by both the poll slice and hard drain.
        let _ = unsafe {
            WaitForSingleObject(
                event,
                duration_millis(remaining.min(DEVICE_IO_POLL_INTERVAL)),
            )
        };
    }
}

fn duration_millis(duration: Duration) -> u32 {
    duration.as_millis().clamp(1, u128::from(u32::MAX)) as u32
}

fn map_device_error(error: io::Error) -> BackendError {
    match error.raw_os_error().map(|value| value as u32) {
        Some(ERROR_JOURNAL_NOT_ACTIVE | ERROR_INVALID_FUNCTION | ERROR_NOT_SUPPORTED) => {
            BackendError::JournalUnavailable
        }
        Some(ERROR_JOURNAL_ENTRY_DELETED | ERROR_JOURNAL_DELETE_IN_PROGRESS | ERROR_HANDLE_EOF) => {
            BackendError::JournalDiscontinuous
        }
        Some(ERROR_ACCESS_DENIED) => BackendError::Unavailable,
        _ => BackendError::Unavailable,
    }
}

fn map_parse_error(_: UsnParseError) -> BackendError {
    BackendError::RecordUnsupported
}

#[derive(Clone, Debug)]
struct ResolvedRecord {
    path: Vec<u16>,
    file_reference: FileReference,
    usn: i64,
    reason: u32,
    is_directory: bool,
}

fn resolve_records(
    root: &PinnedRootState,
    records: &[ParsedUsnRecord],
    context: &mut RequestContext<'_>,
) -> Result<Vec<ResolvedRecord>, BackendError> {
    let histories = reference_histories(records);
    let mut resolved = Vec::with_capacity(records.len());
    for record in records {
        context.bounded_step()?;
        let mut visiting = BTreeSet::new();
        let parent_guid = resolve_reference_path(
            root.volume_handle()
                .map_err(|_| BackendError::RootUnavailable)?,
            record.parent_reference,
            record.usn,
            records,
            &histories,
            &mut visiting,
        )?;
        let mut path = guid_path_to_dos(root, &parent_guid)?;
        if !path.ends_with(&[b'\\' as u16]) {
            path.push(b'\\' as u16);
        }
        path.extend(record.name.encode_utf16());
        if path.len() > MAX_PATH_UTF16_UNITS {
            return Err(BackendError::RecordUnsupported);
        }
        resolved.push(ResolvedRecord {
            path,
            file_reference: record.file_reference,
            usn: record.usn,
            reason: record.reason,
            is_directory: record.file_attributes & FILE_ATTRIBUTE_DIRECTORY != 0,
        });
    }
    Ok(resolved)
}

fn resolve_reference_path(
    volume: HANDLE,
    reference: FileReference,
    before_usn: i64,
    records: &[ParsedUsnRecord],
    histories: &ReferenceHistories,
    visiting: &mut BTreeSet<FileReference>,
) -> Result<Vec<u16>, BackendError> {
    resolve_reference_path_with(
        reference,
        before_usn,
        records,
        histories,
        visiting,
        &|reference| open_reference_guid_path(volume, reference),
        &|mut path, name| {
            if !path.ends_with(&[b'\\' as u16]) {
                path.push(b'\\' as u16);
            }
            path.extend(name.encode_utf16());
            if path.len() > MAX_PATH_UTF16_UNITS {
                return Err(BackendError::RecordUnsupported);
            }
            Ok(path)
        },
    )
    .map_err(|error| match error {
        ReferenceResolutionError::CycleOrDepth => BackendError::RecordUnsupported,
        ReferenceResolutionError::Callback(error) => error,
    })
}

fn open_reference_guid_path(
    volume: HANDLE,
    reference: FileReference,
) -> Result<Vec<u16>, BackendError> {
    let descriptor = match reference {
        FileReference::V2(bytes) => FILE_ID_DESCRIPTOR {
            dwSize: size_of::<FILE_ID_DESCRIPTOR>() as u32,
            Type: FileIdType,
            Anonymous: FILE_ID_DESCRIPTOR_0 {
                FileId: i64::from_le_bytes(bytes),
            },
        },
        FileReference::V3(bytes) => FILE_ID_DESCRIPTOR {
            dwSize: size_of::<FILE_ID_DESCRIPTOR>() as u32,
            Type: ExtendedFileIdType,
            Anonymous: FILE_ID_DESCRIPTOR_0 {
                ExtendedFileId: FILE_ID_128 { Identifier: bytes },
            },
        },
    };
    // SAFETY: descriptor is initialized for its exact V2/V3 identifier width, volume is a pinned
    // verified handle, access is metadata-only, and the returned handle has one owner.
    let raw = unsafe {
        OpenFileById(
            volume,
            &descriptor,
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
        )
    };
    let handle = OwnedHandle::new(raw)?;
    reject_reparse_handle(handle.raw())?;
    final_guid_path(handle.raw())
}

fn reject_reparse_handle(handle: HANDLE) -> Result<(), BackendError> {
    let mut attributes = FILE_ATTRIBUTE_TAG_INFO::default();
    // SAFETY: attributes is a live exact-size output and handle remains live for the query.
    if unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileAttributeTagInfo,
            (&raw mut attributes).cast(),
            u32::try_from(size_of::<FILE_ATTRIBUTE_TAG_INFO>())
                .map_err(|_| BackendError::RecordUnsupported)?,
        )
    } == 0
        || attributes.FileAttributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
    {
        return Err(BackendError::RecordUnsupported);
    }
    Ok(())
}

fn final_guid_path(handle: HANDLE) -> Result<Vec<u16>, BackendError> {
    let mut capacity = 512_usize;
    loop {
        if capacity > MAX_PATH_UTF16_UNITS {
            return Err(BackendError::RecordUnsupported);
        }
        let mut buffer = vec![0_u16; capacity];
        // SAFETY: buffer is initialized, bounded, and live. Returned length is checked before use.
        let length = unsafe {
            GetFinalPathNameByHandleW(
                handle,
                buffer.as_mut_ptr(),
                u32::try_from(buffer.len()).map_err(|_| BackendError::RecordUnsupported)?,
                VOLUME_NAME_GUID,
            )
        } as usize;
        if length == 0 {
            return Err(BackendError::RecordUnsupported);
        }
        if length < buffer.len() {
            buffer.truncate(length);
            String::from_utf16(&buffer).map_err(|_| BackendError::RecordUnsupported)?;
            return Ok(buffer);
        }
        capacity = length
            .checked_add(1)
            .ok_or(BackendError::RecordUnsupported)?;
    }
}

fn guid_path_to_dos(root: &PinnedRootState, guid_path: &[u16]) -> Result<Vec<u16>, BackendError> {
    let volume: Vec<u16> = root.volume_open_path().encode_utf16().collect();
    if guid_path.len() < volume.len()
        || !crate::journal_broker::windows::name::ordinal_equals(
            &guid_path[..volume.len()],
            &volume,
            true,
        )
        .map_err(|_| BackendError::RecordUnsupported)?
    {
        return Err(BackendError::VolumeMismatch);
    }
    let declared = root.canonical_root_utf16();
    let drive = declared.get(..3).ok_or(BackendError::RootUnavailable)?;
    let mut output = drive.to_vec();
    output.extend_from_slice(&guid_path[volume.len()..]);
    Ok(output)
}

fn root_scoped_candidates(
    root: &PinnedRootState,
    records: Vec<ResolvedRecord>,
) -> Result<Vec<BackendJournalCandidate>, BackendError> {
    root_scoped_candidates_with(root.volume_id(), records, |path| path_is_inside(root, path))
}

fn root_scoped_candidates_with(
    volume_id: &str,
    records: Vec<ResolvedRecord>,
    mut is_inside: impl FnMut(&[u16]) -> Result<bool, BackendError>,
) -> Result<Vec<BackendJournalCandidate>, BackendError> {
    let mut candidates = Vec::new();
    let mut old_names = BTreeMap::<FileReference, ResolvedRecord>::new();
    for record in records {
        if record.reason & USN_REASON_RENAME_OLD_NAME != 0 {
            old_names.insert(record.file_reference, record);
            continue;
        }
        if record.reason & USN_REASON_RENAME_NEW_NAME != 0 {
            if let Some(previous) = old_names.remove(&record.file_reference) {
                push_rename_or_scoped_paths(
                    volume_id,
                    &mut candidates,
                    previous,
                    record,
                    &mut is_inside,
                )?;
            } else {
                push_scoped_path(volume_id, &mut candidates, record, &mut is_inside)?;
            }
            continue;
        }
        push_scoped_path(volume_id, &mut candidates, record, &mut is_inside)?;
    }
    for (_, record) in old_names {
        push_scoped_path(volume_id, &mut candidates, record, &mut is_inside)?;
    }
    candidates.sort_by_key(BackendJournalCandidate::usn);
    Ok(candidates)
}

fn push_rename_or_scoped_paths(
    volume_id: &str,
    output: &mut Vec<BackendJournalCandidate>,
    previous: ResolvedRecord,
    current: ResolvedRecord,
    is_inside: &mut impl FnMut(&[u16]) -> Result<bool, BackendError>,
) -> Result<(), BackendError> {
    let previous_inside = is_inside(&previous.path)?;
    let current_inside = is_inside(&current.path)?;
    match (previous_inside, current_inside) {
        (true, true) => output.push(BackendJournalCandidate::copy_from_bounded(
            volume_id,
            &current.path,
            Some(&previous.path),
            &current.file_reference.bytes(),
            current.usn,
            CandidateKind::Rename,
            current.is_directory,
        )?),
        (true, false) => push_scoped_path(volume_id, output, previous, is_inside)?,
        (false, true) => push_scoped_path(volume_id, output, current, is_inside)?,
        (false, false) => {}
    }
    Ok(())
}

fn push_scoped_path(
    volume_id: &str,
    output: &mut Vec<BackendJournalCandidate>,
    record: ResolvedRecord,
    is_inside: &mut impl FnMut(&[u16]) -> Result<bool, BackendError>,
) -> Result<(), BackendError> {
    if !is_inside(&record.path)? {
        return Ok(());
    }
    let kind = if record.is_directory {
        CandidateKind::Subtree
    } else {
        CandidateKind::Path
    };
    output.push(BackendJournalCandidate::copy_from_bounded(
        volume_id,
        &record.path,
        None,
        &record.file_reference.bytes(),
        record.usn,
        kind,
        record.is_directory,
    )?);
    Ok(())
}

fn path_is_inside(root: &PinnedRootState, path: &[u16]) -> Result<bool, BackendError> {
    let path = normalize_backend_path(path).map_err(|_| BackendError::RecordUnsupported)?;
    let root_path = normalize_backend_path(root.canonical_root_utf16())
        .map_err(|_| BackendError::RootUnavailable)?;
    Ok(matches!(
        scope_under_root(&path, &root_path, root.component_semantics())
            .map_err(|_| BackendError::RecordUnsupported)?,
        Some(RootRelativeScope::Root | RootRelativeScope::Relative(_))
    ))
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn new(raw: HANDLE) -> Result<Self, BackendError> {
        if raw.is_null() || raw == INVALID_HANDLE_VALUE {
            Err(BackendError::RecordUnsupported)
        } else {
            Ok(Self(raw))
        }
    }

    fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: construction rejected null and INVALID_HANDLE_VALUE; this unique owner closes
        // the metadata-only handle exactly once.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Ioctl::USN_REASON_FILE_DELETE;

    #[test]
    fn parser_accepts_mixed_v2_and_v3_records() {
        let mut buffer = 80_i64.to_le_bytes().to_vec();
        buffer.extend(v2_record(40, "old.jpg", USN_REASON_FILE_DELETE));
        buffer.extend(v3_record(48, "new.jpg", USN_REASON_RENAME_NEW_NAME));

        let (next, records) = parse_shared_journal_buffer(&buffer).expect("mixed records");

        assert_eq!(next, 80);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].name, "old.jpg");
        assert!(matches!(records[1].file_reference, FileReference::V3(_)));
    }

    #[test]
    fn parser_rejects_out_of_record_name_and_unknown_version() {
        let mut invalid_name = v2_record(40, "photo.jpg", 1);
        invalid_name[58..60].copy_from_slice(&u16::MAX.to_le_bytes());
        let mut buffer = 50_i64.to_le_bytes().to_vec();
        buffer.extend(invalid_name);
        assert!(matches!(
            parse_shared_journal_buffer(&buffer),
            Err(UsnParseError::RecordMalformed)
        ));

        let mut unknown = v2_record(40, "photo.jpg", 1);
        unknown[4..6].copy_from_slice(&4_u16.to_le_bytes());
        let mut buffer = 50_i64.to_le_bytes().to_vec();
        buffer.extend(unknown);
        assert!(matches!(
            parse_shared_journal_buffer(&buffer),
            Err(UsnParseError::VersionUnsupported)
        ));
    }

    #[test]
    fn overlapped_poll_and_cancel_waits_are_strictly_bounded() {
        assert_eq!(duration_millis(Duration::ZERO), 1);
        assert_eq!(duration_millis(Duration::from_millis(25)), 25);
        assert!(DEVICE_IO_POLL_INTERVAL < DEVICE_IO_CANCEL_DRAIN_TIMEOUT);
        assert!(DEVICE_IO_CANCEL_DRAIN_TIMEOUT <= Duration::from_secs(1));
    }

    #[test]
    fn unrelated_volume_storm_returns_bounded_strictly_advancing_pages() {
        let short_records = (1..=7)
            .map(|index| unrelated_record(index * 10, format!("outside-{index}.jpg")))
            .collect::<Vec<_>>();
        assert_unrelated_storm_progress(short_records, 2, usize::MAX, 2);

        let long_records = (1..=4)
            .map(|index| unrelated_record(index * 10, "x".repeat(MAX_PATH_UTF16_UNITS)))
            .collect::<Vec<_>>();
        assert_unrelated_storm_progress(long_records, 16, MAX_SINGLE_RAW_RECORD_EVIDENCE_BYTES, 1);
    }

    #[test]
    #[ignore = "requires the serial Windows R2c-R local reliability wrapper"]
    fn million_unrelated_records_stream_through_bounded_production_parser_pages() {
        const TOTAL_RECORDS: usize = 1_000_000;
        const FIXED_RECORD_BYTES: usize = 64;
        const RECORDS_PER_NATIVE_BUFFER: usize =
            (JOURNAL_BUFFER_BYTES - size_of::<i64>()) / FIXED_RECORD_BYTES;
        const DIRECTORY_REFERENCE: [u8; 8] = [0x44; 8];
        const BASE_REFERENCE: [u8; 8] = [0x99; 8];
        const MAX_FIXTURE_PATH_UTF16_UNITS: usize = 32;
        const HISTORY_NODE_OVERHEAD_BYTES: usize = 128;
        const ALLOCATOR_CAPACITY_MULTIPLIER: usize = 2;
        const PER_ALLOCATION_SLACK_BYTES: usize = 64;
        const MAX_HISTORY_IDENTITIES: usize = RECORDS_PER_NATIVE_BUFFER.div_ceil(2) + 1;
        const MAX_SIMULTANEOUS_ALLOCATIONS: usize =
            7 + RECORDS_PER_NATIVE_BUFFER * 2 + MAX_HISTORY_IDENTITIES * 2;
        const FIXTURE_REQUESTED_CAPACITY_BOUND_BYTES: usize = JOURNAL_BUFFER_BYTES
            + RECORDS_PER_NATIVE_BUFFER * size_of::<ParsedUsnRecord>()
            + RECORDS_PER_NATIVE_BUFFER
            + MAX_HISTORY_IDENTITIES
                * (size_of::<(FileReference, Vec<usize>)>() + HISTORY_NODE_OVERHEAD_BYTES)
            + RECORDS_PER_NATIVE_BUFFER * size_of::<usize>()
            + RECORDS_PER_NATIVE_BUFFER * size_of::<ResolvedRecord>()
            + RECORDS_PER_NATIVE_BUFFER * MAX_FIXTURE_PATH_UTF16_UNITS * size_of::<u16>()
            + size_of::<Vec<ParsedUsnRecord>>()
            + size_of::<ReferenceHistories>()
            + size_of::<Vec<ResolvedRecord>>()
            + size_of::<Vec<BackendJournalCandidate>>()
            + size_of::<BTreeSet<FileReference>>();
        const FIXTURE_CONSERVATIVE_ALLOCATOR_BOUND_BYTES: usize =
            FIXTURE_REQUESTED_CAPACITY_BOUND_BYTES * ALLOCATOR_CAPACITY_MULTIPLIER
                + MAX_SIMULTANEOUS_ALLOCATIONS * PER_ALLOCATION_SLACK_BYTES;

        assert_eq!(RECORDS_PER_NATIVE_BUFFER, 4_095);

        let started = Instant::now();
        let end_usn = i64::try_from(TOTAL_RECORDS + 1).expect("million-record end USN");
        let mut start_usn = 1_i64;
        let mut covered_records = 0_usize;
        let mut pages = 0_usize;
        let mut maximum_native_buffer_length = 0_usize;
        let mut maximum_native_buffer_capacity = 0_usize;
        let mut maximum_retained_records = 0_usize;
        let mut maximum_retained_capacity_bytes = 0_usize;
        let mut maximum_history_capacity_bytes = 0_usize;
        let mut maximum_resolved_capacity_bytes = 0_usize;
        let mut maximum_simultaneous_requested_capacity_bytes = 0_usize;
        let mut maximum_conservative_allocator_bound_bytes = 0_usize;
        let mut maximum_history_reuse = 0_usize;
        let mut total_scope_checks = 0_usize;
        while start_usn < end_usn {
            let page_end = start_usn
                .saturating_add(
                    i64::try_from(RECORDS_PER_NATIVE_BUFFER).expect("native page record count"),
                )
                .min(end_usn);
            let mut buffer = vec![0_u8; JOURNAL_BUFFER_BYTES];
            buffer[..size_of::<i64>()].copy_from_slice(&page_end.to_le_bytes());
            let mut buffer_length = size_of::<i64>();
            for (local_index, usn) in (start_usn..page_end).enumerate() {
                let is_directory_history = local_index.is_multiple_of(2);
                let directory_name = if (local_index / 2).is_multiple_of(2) {
                    "a"
                } else {
                    "b"
                };
                let record = v2_record_with_references(
                    usn,
                    if is_directory_history {
                        directory_name
                    } else {
                        "x"
                    },
                    USN_REASON_FILE_DELETE,
                    if is_directory_history {
                        DIRECTORY_REFERENCE
                    } else {
                        usn.to_le_bytes()
                    },
                    if is_directory_history {
                        BASE_REFERENCE
                    } else {
                        DIRECTORY_REFERENCE
                    },
                    is_directory_history,
                );
                assert_eq!(record.len(), FIXED_RECORD_BYTES);
                let record_end = buffer_length
                    .checked_add(record.len())
                    .expect("fixed native buffer offset");
                buffer[buffer_length..record_end].copy_from_slice(&record);
                buffer_length = record_end;
            }
            buffer.truncate(buffer_length);
            assert_eq!(buffer.capacity(), JOURNAL_BUFFER_BYTES);
            maximum_native_buffer_length = maximum_native_buffer_length.max(buffer.len());
            maximum_native_buffer_capacity = maximum_native_buffer_capacity.max(buffer.capacity());
            let (records, covered_until_usn) = read_single_native_journal_buffer_with(
                start_usn,
                end_usn,
                MAX_RAW_RECORDS_PER_PAGE,
                MAX_RAW_PAGE_RETAINED_BYTES,
                |physical_start_usn| {
                    assert_eq!(physical_start_usn, start_usn);
                    Ok(buffer)
                },
            )
            .expect("bounded production parser page");
            assert_eq!(covered_until_usn, page_end);
            assert!(covered_until_usn > start_usn);
            for (offset, record) in records.iter().enumerate() {
                assert_eq!(
                    record.usn,
                    start_usn + i64::try_from(offset).expect("page USN offset"),
                    "the production parser must retain strict per-USN order"
                );
            }
            let retained_capacity_bytes = records
                .capacity()
                .checked_mul(size_of::<ParsedUsnRecord>())
                .and_then(|total| {
                    records.iter().try_fold(total, |total, record| {
                        total.checked_add(record.name.capacity())
                    })
                })
                .expect("bounded retained record capacity");
            maximum_retained_records = maximum_retained_records.max(records.len());
            maximum_retained_capacity_bytes =
                maximum_retained_capacity_bytes.max(retained_capacity_bytes);

            let histories = reference_histories(&records);
            let history_reuse = histories
                .get(&FileReference::V2(DIRECTORY_REFERENCE))
                .map_or(0, Vec::len);
            maximum_history_reuse = maximum_history_reuse.max(history_reuse);
            assert_eq!(history_reuse, records.len().div_ceil(2));
            let history_capacity_bytes = histories
                .len()
                .checked_mul(size_of::<(FileReference, Vec<usize>)>() + HISTORY_NODE_OVERHEAD_BYTES)
                .and_then(|total| {
                    histories.values().try_fold(total, |total, indices| {
                        total.checked_add(indices.capacity().saturating_mul(size_of::<usize>()))
                    })
                })
                .expect("bounded reference-history capacity");
            maximum_history_capacity_bytes =
                maximum_history_capacity_bytes.max(history_capacity_bytes);

            let historical_appends = std::cell::Cell::new(0_usize);
            let mut resolved = Vec::with_capacity(records.len());
            for (local_index, record) in records.iter().enumerate() {
                let mut visiting = BTreeSet::new();
                let mut path = resolve_reference_path_with(
                    record.parent_reference,
                    record.usn,
                    &records,
                    &histories,
                    &mut visiting,
                    &|_| Ok::<Vec<u16>, ()>(r"C:\Unrelated".encode_utf16().collect()),
                    &|mut path, name| {
                        historical_appends.set(historical_appends.get().saturating_add(1));
                        if !path.ends_with(&[b'\\' as u16]) {
                            path.push(b'\\' as u16);
                        }
                        path.extend(name.encode_utf16());
                        Ok(path)
                    },
                )
                .expect("resolve reused production reference history");
                if !path.ends_with(&[b'\\' as u16]) {
                    path.push(b'\\' as u16);
                }
                path.extend(record.name.encode_utf16());
                assert!(path.len() <= MAX_FIXTURE_PATH_UTF16_UNITS);
                let resolved_text = String::from_utf16(&path).expect("resolved fixture path");
                if local_index.is_multiple_of(2) {
                    assert!(
                        resolved_text.ends_with(if (local_index / 2).is_multiple_of(2) {
                            r"\a"
                        } else {
                            r"\b"
                        })
                    );
                } else {
                    assert!(resolved_text.ends_with(
                        if ((local_index - 1) / 2).is_multiple_of(2) {
                            r"\a\x"
                        } else {
                            r"\b\x"
                        }
                    ));
                }
                resolved.push(ResolvedRecord {
                    path,
                    file_reference: record.file_reference,
                    usn: record.usn,
                    reason: record.reason,
                    is_directory: record.file_attributes & FILE_ATTRIBUTE_DIRECTORY != 0,
                });
            }
            assert_eq!(historical_appends.get(), records.len() / 2);
            let resolved_capacity_bytes = resolved
                .capacity()
                .checked_mul(size_of::<ResolvedRecord>())
                .and_then(|total| {
                    resolved.iter().try_fold(total, |total, record| {
                        total.checked_add(record.path.capacity().saturating_mul(size_of::<u16>()))
                    })
                })
                .expect("bounded resolved record capacity");
            maximum_resolved_capacity_bytes =
                maximum_resolved_capacity_bytes.max(resolved_capacity_bytes);
            let simultaneous_requested_capacity_bytes = JOURNAL_BUFFER_BYTES
                .checked_add(retained_capacity_bytes)
                .and_then(|total| total.checked_add(history_capacity_bytes))
                .and_then(|total| total.checked_add(resolved_capacity_bytes))
                .and_then(|total| total.checked_add(size_of::<Vec<ParsedUsnRecord>>()))
                .and_then(|total| total.checked_add(size_of::<ReferenceHistories>()))
                .and_then(|total| total.checked_add(size_of::<Vec<ResolvedRecord>>()))
                .and_then(|total| total.checked_add(size_of::<Vec<BackendJournalCandidate>>()))
                .and_then(|total| total.checked_add(size_of::<BTreeSet<FileReference>>()))
                .expect("bounded simultaneous requested capacity");
            let allocation_count = 7_usize
                .checked_add(records.len().saturating_mul(2))
                .and_then(|count| count.checked_add(histories.len().saturating_mul(2)))
                .expect("bounded allocation count");
            let conservative_allocator_bound_bytes = simultaneous_requested_capacity_bytes
                .checked_mul(ALLOCATOR_CAPACITY_MULTIPLIER)
                .and_then(|total| {
                    total.checked_add(allocation_count.saturating_mul(PER_ALLOCATION_SLACK_BYTES))
                })
                .expect("bounded conservative allocator estimate");
            assert!(
                simultaneous_requested_capacity_bytes <= FIXTURE_REQUESTED_CAPACITY_BOUND_BYTES
            );
            assert!(
                conservative_allocator_bound_bytes <= FIXTURE_CONSERVATIVE_ALLOCATOR_BOUND_BYTES
            );
            maximum_simultaneous_requested_capacity_bytes =
                maximum_simultaneous_requested_capacity_bytes
                    .max(simultaneous_requested_capacity_bytes);
            maximum_conservative_allocator_bound_bytes =
                maximum_conservative_allocator_bound_bytes.max(conservative_allocator_bound_bytes);

            let scope_checks = std::cell::Cell::new(0_usize);
            let candidates = root_scoped_candidates_with("volume-storm", resolved, |_| {
                scope_checks.set(scope_checks.get().saturating_add(1));
                Ok(false)
            })
            .expect("filter root-external million-record page");
            assert_eq!(scope_checks.get(), records.len());
            total_scope_checks = total_scope_checks
                .checked_add(scope_checks.get())
                .expect("million-record scope checks");
            assert!(
                candidates.is_empty(),
                "root-external record names or paths escaped into broker candidates"
            );
            covered_records = covered_records
                .checked_add(records.len())
                .expect("million-record coverage count");
            pages = pages.saturating_add(1);
            start_usn = covered_until_usn;
        }

        let elapsed = started.elapsed();
        eprintln!(
            "R2c-R million-backlog records={TOTAL_RECORDS} covered_records={covered_records} pages={pages} production_records_per_native_buffer={RECORDS_PER_NATIVE_BUFFER} retained_frames_max=1 retained_records_max={maximum_retained_records} native_buffer_length_max={maximum_native_buffer_length} native_buffer_capacity={maximum_native_buffer_capacity} retained_record_capacity_bytes_max={maximum_retained_capacity_bytes} reference_history_capacity_bytes_max={maximum_history_capacity_bytes} resolved_record_capacity_bytes_max={maximum_resolved_capacity_bytes} reused_identity_history_records_max={maximum_history_reuse} simultaneous_requested_capacity_bytes_max={maximum_simultaneous_requested_capacity_bytes} allocator_capacity_multiplier={ALLOCATOR_CAPACITY_MULTIPLIER} per_allocation_slack_bytes={PER_ALLOCATION_SLACK_BYTES} conservative_allocator_bound_bytes_max={maximum_conservative_allocator_bound_bytes} fixture_allocator_bound_bytes={FIXTURE_CONSERVATIVE_ALLOCATOR_BOUND_BYTES} root_scope_checks={total_scope_checks} emitted_candidates=0 elapsed_ms={}",
            elapsed.as_millis()
        );
        assert_eq!(covered_records, TOTAL_RECORDS);
        assert_eq!(total_scope_checks, TOTAL_RECORDS);
        assert_eq!(pages, TOTAL_RECORDS.div_ceil(RECORDS_PER_NATIVE_BUFFER));
        assert!(maximum_retained_records <= RECORDS_PER_NATIVE_BUFFER);
        assert!(maximum_retained_records <= MAX_RAW_RECORDS_PER_PAGE);
        assert_eq!(maximum_native_buffer_capacity, JOURNAL_BUFFER_BYTES);
        assert!(maximum_native_buffer_length <= JOURNAL_BUFFER_BYTES);
        assert!(
            maximum_retained_capacity_bytes
                <= MAX_RAW_PAGE_RETAINED_BYTES + RECORDS_PER_NATIVE_BUFFER
        );
        assert!(maximum_history_reuse > 1);
        assert!(
            maximum_conservative_allocator_bound_bytes
                <= FIXTURE_CONSERVATIVE_ALLOCATOR_BOUND_BYTES
        );
        assert_eq!(start_usn, end_usn);
    }

    #[test]
    fn native_page_seam_invokes_one_physical_read_and_returns_partial_proof() {
        let mut buffer = 80_i64.to_le_bytes().to_vec();
        buffer.extend(v2_record(40, "first.jpg", USN_REASON_FILE_DELETE));
        buffer.extend(v2_record(48, "second.jpg", USN_REASON_FILE_DELETE));
        let physical_reads = std::cell::Cell::new(0_usize);

        let (records, covered_until_usn) =
            read_single_native_journal_buffer_with(40, 80, 1, usize::MAX, |_| {
                physical_reads.set(physical_reads.get() + 1);
                Ok(buffer)
            })
            .expect("single physical read");

        assert_eq!(physical_reads.get(), 1);
        assert_eq!(records.len(), 1);
        assert_eq!(covered_until_usn, 48);
    }

    #[test]
    fn per_root_start_filters_before_containment_and_is_order_independent() {
        let mut first = vec![Ok(Vec::new()), Ok(Vec::new())];
        let mut calls = [0_usize; 2];
        let inside = roots_for_path_with(
            2,
            &[10, 50],
            &mut first,
            &"C:\\Pictures\\photo.jpg".encode_utf16().collect::<Vec<_>>(),
            40,
            |index, _| {
                calls[index] += 1;
                Ok(true)
            },
        );
        assert_eq!(inside, vec![0]);
        assert_eq!(calls, [1, 0]);

        let mut swapped = vec![Ok(Vec::new()), Ok(Vec::new())];
        let mut swapped_calls = [0_usize; 2];
        let inside = roots_for_path_with(
            2,
            &[50, 10],
            &mut swapped,
            &"C:\\Pictures\\photo.jpg".encode_utf16().collect::<Vec<_>>(),
            40,
            |index, _| {
                swapped_calls[index] += 1;
                Ok(true)
            },
        );
        assert_eq!(inside, vec![1]);
        assert_eq!(swapped_calls, [0, 1]);
    }

    #[test]
    fn endpoint_uncertainty_keeps_only_the_safe_prefix_and_durable_old() {
        let path = "C:\\A\\old.jpg".encode_utf16().collect::<Vec<_>>();
        let mut status = vec![Ok(Vec::new()), Ok(Vec::new())];
        let mut barrier = None;
        let mut root_barriers = vec![None, None];
        let (owners, uncertain) = roots_for_rename_path_with(
            2,
            &[10, 10],
            &mut status,
            &path,
            20,
            &mut barrier,
            &mut root_barriers,
            |index, _| {
                if index == 0 {
                    Err(BackendError::RootUnavailable)
                } else {
                    Ok(false)
                }
            },
        );
        assert!(owners.is_empty());
        assert!(uncertain);
        assert_eq!(barrier, Some(20));
        assert_eq!(root_barriers, vec![Some(20), None]);

        let prefix = SemanticEvent {
            usn: 15,
            candidates: vec![(1, test_candidate("volume-a", "C:\\B\\prefix.jpg", 15))],
            ..SemanticEvent::default()
        };
        let unsafe_new = SemanticEvent {
            usn: 25,
            candidates: vec![(1, test_candidate("volume-a", "C:\\B\\new.jpg", 25))],
            ..SemanticEvent::default()
        };
        let (covered, _, pending) = apply_semantic_limits(
            &mut status,
            vec![prefix, unsafe_new],
            30,
            barrier,
            &[10, 10],
            16,
            usize::MAX,
        )
        .expect("safe source-failure prefix");
        assert_eq!(covered, 20);
        assert!(pending.is_empty());
        assert_eq!(status[1].as_ref().expect("healthy target").len(), 1);

        let mut target_status = vec![Ok(Vec::new()), Ok(Vec::new())];
        let old = SemanticEvent {
            usn: 20,
            pending: Some(BackendPendingRename {
                root_index: 0,
                previous_absolute_path_utf16: path,
                file_reference: vec![7; 8],
                old_usn: 20,
                is_directory: false,
            }),
            ..SemanticEvent::default()
        };
        let (covered, handoffs, pending) = apply_semantic_limits(
            &mut target_status,
            vec![old],
            30,
            Some(25),
            &[10, 10],
            16,
            usize::MAX,
        )
        .expect("target-failure durable OLD prefix");
        assert_eq!(covered, 25);
        assert!(handoffs.is_empty());
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].old_usn, 20);
    }

    #[test]
    fn uncertainty_barrier_uses_first_rename_at_or_after_each_root_start() {
        let mut buffer = 40_i64.to_le_bytes().to_vec();
        buffer.extend(v2_record(20, "old.jpg", USN_REASON_RENAME_OLD_NAME));
        buffer.extend(v2_record(25, "plain.jpg", USN_REASON_FILE_DELETE));
        buffer.extend(v2_record(30, "new.jpg", USN_REASON_RENAME_NEW_NAME));
        let (_, records) = parse_shared_journal_buffer(&buffer).expect("rename fixture");

        assert_eq!(first_rename_usn_at_or_after(&records, 10), Some(20));
        assert_eq!(first_rename_usn_at_or_after(&records, 25), Some(30));
        assert_eq!(first_rename_usn_at_or_after(&records, 31), None);
    }

    #[test]
    fn durable_pending_old_pairs_after_unrelated_records_and_native_buffer_boundary() {
        let file_reference = FileReference::V2([9; 8]);
        let mut first_status = vec![Ok(Vec::new())];
        let first_page = vec![SemanticEvent {
            usn: 20,
            pending: Some(BackendPendingRename {
                root_index: 0,
                previous_absolute_path_utf16: "C:\\A\\old.jpg".encode_utf16().collect(),
                file_reference: file_reference.bytes(),
                old_usn: 20,
                is_directory: false,
            }),
            ..SemanticEvent::default()
        }];
        let (_, _, durable) = apply_semantic_limits(
            &mut first_status,
            first_page,
            40,
            None,
            &[10],
            16,
            usize::MAX,
        )
        .expect("persist OLD page");
        assert_eq!(durable.len(), 1);

        let mut second_status = vec![Ok(Vec::new()), Ok(Vec::new())];
        let mut second_page = (41..=1_000)
            .map(|usn| SemanticEvent {
                usn,
                candidates: vec![(
                    1,
                    test_candidate("volume-a", &format!("C:\\B\\noise-{usn}.jpg"), usn),
                )],
                ..SemanticEvent::default()
            })
            .collect::<Vec<_>>();
        push_pair_event_with_current_roots(
            &["volume-a", "volume-a"],
            &mut second_status,
            &mut second_page,
            RenamePairEvent {
                previous_path: durable[0].previous_absolute_path_utf16.clone(),
                previous_usn: durable[0].old_usn,
                previous_roots: Vec::new(),
                previous_pending_index: Some(0),
                current: ResolvedRecord {
                    path: "C:\\B\\new.jpg".encode_utf16().collect(),
                    file_reference,
                    usn: 1_001,
                    reason: USN_REASON_RENAME_NEW_NAME,
                    is_directory: false,
                },
            },
            vec![1],
            false,
        )
        .expect("pair carried OLD with later NEW");
        second_page.sort_by_key(|event| event.usn);
        let (_, handoffs, pending) = apply_semantic_limits(
            &mut second_status,
            second_page,
            1_100,
            None,
            &[40, 40],
            1_024,
            usize::MAX,
        )
        .expect("bounded second page");
        assert!(pending.is_empty());
        assert_eq!(handoffs.len(), 1);
        assert_eq!(handoffs[0].previous_pending_index, Some(0));
        assert_eq!(handoffs[0].current_root_index, 1);
    }

    #[test]
    fn carried_same_root_rename_is_one_semantic_candidate() {
        let mut status = vec![Ok(Vec::new())];
        let mut events = Vec::new();
        push_pair_event_with_current_roots(
            &["volume-a"],
            &mut status,
            &mut events,
            RenamePairEvent {
                previous_path: "C:\\A\\old.jpg".encode_utf16().collect(),
                previous_usn: 20,
                previous_roots: vec![0],
                previous_pending_index: Some(0),
                current: ResolvedRecord {
                    path: "C:\\A\\new.jpg".encode_utf16().collect(),
                    file_reference: FileReference::V2([5; 8]),
                    usn: 30,
                    reason: USN_REASON_RENAME_NEW_NAME,
                    is_directory: false,
                },
            },
            vec![0],
            false,
        )
        .expect("same-root carried pair");
        assert_eq!(events.len(), 1);
        assert!(events[0].handoff.is_none());
        assert_eq!(events[0].candidates.len(), 1);
        assert!(matches!(
            events[0].candidates[0].1.kind(),
            CandidateKind::Rename
        ));
    }

    #[test]
    fn semantic_budget_stops_before_whole_rename_and_root_order_cannot_starve() {
        let first = SemanticEvent {
            usn: 20,
            candidates: vec![(0, test_candidate("volume-a", "C:\\A\\first.jpg", 20))],
            ..SemanticEvent::default()
        };
        let rename = SemanticEvent {
            usn: 30,
            candidates: vec![(
                1,
                BackendJournalCandidate::copy_from_bounded(
                    "volume-a",
                    &"C:\\B\\new.jpg".encode_utf16().collect::<Vec<_>>(),
                    Some(&"C:\\B\\old.jpg".encode_utf16().collect::<Vec<_>>()),
                    &[7; 8],
                    30,
                    CandidateKind::Rename,
                    false,
                )
                .expect("rename candidate"),
            )],
            ..SemanticEvent::default()
        };
        for (first_root, second_root) in [(0, 1), (1, 0)] {
            let mut status = vec![Ok(Vec::new()), Ok(Vec::new())];
            let mut first_event = first.clone_for_test(first_root);
            let mut rename_event = rename.clone_for_test(second_root);
            first_event.usn = 20;
            rename_event.usn = 30;
            let (covered, handoffs, pending) = apply_semantic_limits(
                &mut status,
                vec![first_event, rename_event],
                50,
                None,
                &[10, 10],
                1,
                usize::MAX,
            )
            .expect("fair semantic page");
            assert_eq!(covered, 30);
            assert!(handoffs.is_empty());
            assert!(pending.is_empty());
            assert_eq!(
                status
                    .iter()
                    .filter_map(|outcome| outcome.as_ref().ok())
                    .map(Vec::len)
                    .sum::<usize>(),
                1
            );
        }
    }

    #[test]
    fn oversized_first_semantic_record_fails_only_its_root_without_livelock() {
        let long_path = format!("C:\\A\\{}.jpg", "x".repeat(2_048));
        let event = SemanticEvent {
            usn: 10,
            candidates: vec![(0, test_candidate("volume-a", &long_path, 10))],
            ..SemanticEvent::default()
        };
        let mut status = vec![Ok(Vec::new()), Ok(Vec::new())];
        let (covered, _, _) =
            apply_semantic_limits(&mut status, vec![event], 50, None, &[10, 10], 16, 64)
                .expect("isolate oversized root");
        assert_eq!(covered, 50);
        assert!(matches!(
            status[0],
            Err(BackendError::EvidenceLimitExceeded)
        ));
        assert!(status[1].is_ok());
    }

    impl SemanticEvent {
        fn clone_for_test(&self, root_index: usize) -> Self {
            Self {
                usn: self.usn,
                candidates: self
                    .candidates
                    .iter()
                    .map(|(_, candidate)| (root_index, candidate.clone()))
                    .collect(),
                handoff: None,
                pending: None,
            }
        }
    }

    fn test_candidate(volume_id: &str, path: &str, usn: i64) -> BackendJournalCandidate {
        BackendJournalCandidate::copy_from_bounded(
            volume_id,
            &path.encode_utf16().collect::<Vec<_>>(),
            None,
            &[4; 8],
            usn,
            CandidateKind::Path,
            false,
        )
        .expect("test candidate")
    }

    fn assert_unrelated_storm_progress(
        records: Vec<ParsedUsnRecord>,
        max_records: usize,
        max_evidence_bytes: usize,
        maximum_page_records: usize,
    ) {
        let mut start_usn = records.first().expect("storm record").usn;
        let end_usn = records.last().expect("storm record").usn + 10;
        let mut observed = 0_usize;
        while start_usn < end_usn {
            let mut page_records = Vec::new();
            let mut page_bytes = 0_usize;
            let first_unretained = retain_bounded_raw_records(
                &mut page_records,
                &mut page_bytes,
                records
                    .iter()
                    .filter(|record| record.usn >= start_usn)
                    .cloned()
                    .collect(),
                end_usn,
                max_records,
                max_evidence_bytes,
            )
            .expect("bounded unrelated page");
            assert!(!page_records.is_empty());
            assert!(page_records.len() <= maximum_page_records);
            assert!(page_bytes <= max_evidence_bytes);
            let candidates = root_scoped_candidates_with(
                "volume-storm",
                page_records
                    .clone()
                    .into_iter()
                    .map(|record| ResolvedRecord {
                        path: format!(r"C:\Unrelated\{}", record.usn)
                            .encode_utf16()
                            .collect(),
                        file_reference: record.file_reference,
                        usn: record.usn,
                        reason: record.reason,
                        is_directory: false,
                    })
                    .collect(),
                |_| Ok(false),
            )
            .expect("filter root-external records");
            assert!(candidates.is_empty());
            observed += page_records.len();
            let covered_until_usn = first_unretained.unwrap_or(end_usn);
            assert!(covered_until_usn > start_usn);
            assert!(covered_until_usn <= end_usn);
            if covered_until_usn < end_usn {
                assert!(!page_records.is_empty());
            }
            start_usn = covered_until_usn;
        }
        assert_eq!(observed, records.len());
    }

    fn unrelated_record(usn: i64, name: String) -> ParsedUsnRecord {
        let value = u8::try_from(usn / 10).expect("bounded test reference");
        ParsedUsnRecord {
            file_reference: FileReference::V2([value; 8]),
            parent_reference: FileReference::V2([value.wrapping_add(20); 8]),
            usn,
            timestamp_100ns: 0,
            reason: USN_REASON_FILE_DELETE,
            file_attributes: 0,
            name,
        }
    }

    fn v2_record(usn: i64, name: &str, reason: u32) -> Vec<u8> {
        record(2, 60, usn, name, reason, 8, 16, 24, 40, 52, 56, 58)
    }

    fn v2_record_with_references(
        usn: i64,
        name: &str,
        reason: u32,
        file_reference: [u8; 8],
        parent_reference: [u8; 8],
        is_directory: bool,
    ) -> Vec<u8> {
        let mut record = v2_record(usn, name, reason);
        record[8..16].copy_from_slice(&file_reference);
        record[16..24].copy_from_slice(&parent_reference);
        record[52..56].copy_from_slice(
            &(if is_directory {
                FILE_ATTRIBUTE_DIRECTORY
            } else {
                0
            })
            .to_le_bytes(),
        );
        record
    }

    fn v3_record(usn: i64, name: &str, reason: u32) -> Vec<u8> {
        record(3, 76, usn, name, reason, 8, 24, 40, 56, 68, 72, 74)
    }

    #[allow(clippy::too_many_arguments)]
    fn record(
        version: u16,
        fixed: usize,
        usn: i64,
        name: &str,
        reason: u32,
        file_offset: usize,
        parent_offset: usize,
        usn_offset: usize,
        reason_offset: usize,
        attributes_offset: usize,
        name_length_offset: usize,
        name_offset_offset: usize,
    ) -> Vec<u8> {
        let name: Vec<u16> = name.encode_utf16().collect();
        let raw_length = fixed + name.len() * 2;
        let length = raw_length.next_multiple_of(8);
        let mut record = vec![0_u8; length];
        record[..4].copy_from_slice(&(length as u32).to_le_bytes());
        record[4..6].copy_from_slice(&version.to_le_bytes());
        for (index, byte) in (1_u8..).take(if version == 2 { 8 } else { 16 }).enumerate() {
            record[file_offset + index] = byte;
            record[parent_offset + index] = byte.wrapping_add(20);
        }
        record[usn_offset..usn_offset + 8].copy_from_slice(&usn.to_le_bytes());
        record[reason_offset..reason_offset + 4].copy_from_slice(&reason.to_le_bytes());
        record[attributes_offset..attributes_offset + 4].copy_from_slice(&0_u32.to_le_bytes());
        record[name_length_offset..name_length_offset + 2]
            .copy_from_slice(&((name.len() * 2) as u16).to_le_bytes());
        record[name_offset_offset..name_offset_offset + 2]
            .copy_from_slice(&(fixed as u16).to_le_bytes());
        for (index, unit) in name.into_iter().enumerate() {
            let offset = fixed + index * 2;
            record[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }
        record
    }
}
