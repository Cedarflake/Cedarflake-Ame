use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, c_void};
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};

use blake3::Hasher;
use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ExtendedFileIdType, FILE_ATTRIBUTE_DIRECTORY, FILE_FLAG_BACKUP_SEMANTICS,
    FILE_ID_128, FILE_ID_DESCRIPTOR, FILE_ID_DESCRIPTOR_0, FILE_READ_ATTRIBUTES, FILE_SHARE_DELETE,
    FILE_SHARE_READ, FILE_SHARE_WRITE, FileIdType, GetFinalPathNameByHandleW,
    GetVolumeInformationW, GetVolumeNameForVolumeMountPointW, GetVolumePathNameW, OPEN_EXISTING,
    OpenFileById, VOLUME_NAME_GUID,
};
use windows_sys::Win32::System::IO::DeviceIoControl;
use windows_sys::Win32::System::Ioctl::{
    FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V1, USN_JOURNAL_DATA_V2,
    USN_REASON_FILE_CREATE, USN_REASON_FILE_DELETE, USN_REASON_RENAME_NEW_NAME,
    USN_REASON_RENAME_OLD_NAME,
};

use crate::domain::{
    IncrementalCatalogRoot, LibraryChangeCatchUpBatch, LibraryChangeCatchUpCheckpoint,
    LibraryChangeCatchUpEvidence, LibraryChangeCatchUpLimits, LibraryChangeCatchUpRootResult,
    LibraryChangeObservation, LibraryChangeObservationKind, LibraryChangeOrigin,
    LibraryChangeScope, ScanError,
};
use crate::ports::LibraryChangeCatchUpSource;

const CATCH_UP_SOURCE: &str = "windows_usn_v1";
const JOURNAL_BUFFER_BYTES: usize = 64 * 1_024;
const MAX_PATH_UTF16: usize = 32_768;
const WINDOWS_TO_UNIX_EPOCH_100NS: i64 = 116_444_736_000_000_000;
const HUNDRED_NS_PER_MILLISECOND: i64 = 10_000;

pub(crate) struct WindowsUsnCatchUpSource<Backend = Win32UsnBackend> {
    backend: Backend,
}

impl WindowsUsnCatchUpSource<Win32UsnBackend> {
    pub(crate) const fn production() -> Self {
        Self {
            backend: Win32UsnBackend,
        }
    }
}

impl<Backend> LibraryChangeCatchUpSource for WindowsUsnCatchUpSource<Backend>
where
    Backend: UsnJournalBackend,
{
    fn read_changes(
        &self,
        roots: &[IncrementalCatalogRoot],
        checkpoints: &[LibraryChangeCatchUpCheckpoint],
        observed_unix_ms: i64,
        limits: LibraryChangeCatchUpLimits,
        cancelled: &AtomicBool,
    ) -> Result<LibraryChangeCatchUpBatch, ScanError> {
        if !limits.is_valid() {
            return Err(ScanError::new(
                "library_change_catch_up_limits_invalid",
                "Windows journal catch-up limits must stay within their absolute bounds",
            ));
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(catch_up_cancelled());
        }
        let stored = checkpoints
            .iter()
            .map(|checkpoint| (checkpoint.volume_id.as_str(), checkpoint))
            .collect::<BTreeMap<_, _>>();
        let mut groups = BTreeMap::<String, Vec<ResolvedRoot>>::new();
        let mut results = Vec::with_capacity(roots.len());
        for root in roots {
            match self.backend.describe_root(root) {
                Ok(resolved) if filesystem_supports_usn(&resolved.filesystem) => {
                    groups
                        .entry(resolved.volume.volume_id.clone())
                        .or_default()
                        .push(resolved);
                }
                Ok(_) => results.push(fallback_root(root, "usn_filesystem_unsupported", None)),
                Err(error) => results.push(fallback_root(root, &error.code, None)),
            }
        }

        let mut checkpoint_candidates = Vec::new();
        for resolved_roots in groups.values_mut() {
            if cancelled.load(Ordering::Acquire) {
                return Err(catch_up_cancelled());
            }
            resolved_roots.sort_by(|left, right| left.root.root_id.cmp(&right.root.root_id));
            let volume = &resolved_roots[0].volume;
            let metadata = match self.backend.query_journal(volume) {
                Ok(metadata) => metadata,
                Err(error) => {
                    results.extend(
                        resolved_roots
                            .iter()
                            .map(|resolved| fallback_root(&resolved.root, &error.code, None)),
                    );
                    continue;
                }
            };
            let root_set_fingerprint = root_set_fingerprint(resolved_roots);
            let catalog_revision = resolved_roots
                .iter()
                .map(|resolved| resolved.root.catalog_revision)
                .max()
                .unwrap_or(0);
            let evidence = catch_up_evidence(volume, metadata.journal_id, metadata.next_usn);
            let checkpoint = LibraryChangeCatchUpCheckpoint {
                volume_id: volume.volume_id.clone(),
                journal_id: metadata.journal_id.to_string(),
                next_usn: metadata.next_usn.to_string(),
                root_set_fingerprint: root_set_fingerprint.clone(),
                catalog_revision,
                updated_unix_ms: observed_unix_ms.max(0),
            };
            let continuity = stored
                .get(volume.volume_id.as_str())
                .copied()
                .and_then(|stored| {
                    validate_continuity(stored, &metadata, &root_set_fingerprint, catalog_revision)
                        .ok()
                        .map(|()| stored)
                });
            let Some(continuity) = continuity else {
                let code = stored
                    .get(volume.volume_id.as_str())
                    .map_or("usn_checkpoint_missing", |_| "usn_continuity_invalid");
                results.extend(
                    resolved_roots.iter().map(|resolved| {
                        fallback_root(&resolved.root, code, Some(evidence.clone()))
                    }),
                );
                checkpoint_candidates.push(checkpoint);
                continue;
            };
            let start_usn = parse_checkpoint_usn(continuity)?;
            let changes = match self.backend.read_journal(
                volume,
                &metadata,
                JournalReadBounds {
                    start_usn,
                    end_usn: metadata.next_usn,
                    max_records: limits.max_records_per_volume,
                    max_evidence_bytes: limits.max_evidence_bytes_per_volume,
                },
                cancelled,
            ) {
                Ok(changes) => changes,
                Err(error) => {
                    results.extend(resolved_roots.iter().map(|resolved| {
                        fallback_root(&resolved.root, &error.code, Some(evidence.clone()))
                    }));
                    checkpoint_candidates.push(checkpoint);
                    continue;
                }
            };
            if changes.len() > limits.max_records_per_volume {
                results.extend(resolved_roots.iter().map(|resolved| {
                    fallback_root(
                        &resolved.root,
                        "usn_record_limit_exceeded",
                        Some(evidence.clone()),
                    )
                }));
                checkpoint_candidates.push(checkpoint);
                continue;
            }
            if resolved_changes_bytes(&changes)
                .is_none_or(|bytes| bytes > limits.max_evidence_bytes_per_volume)
            {
                results.extend(resolved_roots.iter().map(|resolved| {
                    fallback_root(
                        &resolved.root,
                        "usn_evidence_bytes_limit_exceeded",
                        Some(evidence.clone()),
                    )
                }));
                checkpoint_candidates.push(checkpoint);
                continue;
            }
            match self.backend.query_journal(volume) {
                Ok(after)
                    if after.journal_id == metadata.journal_id
                        && after.first_usn <= start_usn
                        && after.next_usn >= metadata.next_usn => {}
                Ok(_) => {
                    results.extend(resolved_roots.iter().map(|resolved| {
                        fallback_root(
                            &resolved.root,
                            "usn_continuity_changed_during_read",
                            Some(evidence.clone()),
                        )
                    }));
                    checkpoint_candidates.push(checkpoint);
                    continue;
                }
                Err(error) => {
                    results.extend(resolved_roots.iter().map(|resolved| {
                        fallback_root(&resolved.root, &error.code, Some(evidence.clone()))
                    }));
                    checkpoint_candidates.push(checkpoint);
                    continue;
                }
            }
            match distribute_changes(
                resolved_roots,
                changes,
                limits.max_observations_per_root,
                &evidence,
            ) {
                Ok(mut root_results) => results.append(&mut root_results),
                Err(error) => results.extend(resolved_roots.iter().map(|resolved| {
                    fallback_root(&resolved.root, &error.code, Some(evidence.clone()))
                })),
            }
            checkpoint_candidates.push(checkpoint);
        }
        results.sort_by(|left, right| left.root_id.cmp(&right.root_id));
        Ok(LibraryChangeCatchUpBatch {
            roots: results,
            checkpoints: checkpoint_candidates,
        })
    }
}

trait UsnJournalBackend: Send + Sync + 'static {
    fn describe_root(&self, root: &IncrementalCatalogRoot) -> Result<ResolvedRoot, ScanError>;
    fn query_journal(&self, volume: &VolumeDescriptor) -> Result<JournalMetadata, ScanError>;
    fn read_journal(
        &self,
        volume: &VolumeDescriptor,
        metadata: &JournalMetadata,
        bounds: JournalReadBounds,
        cancelled: &AtomicBool,
    ) -> Result<Vec<ResolvedJournalChange>, ScanError>;
}

#[derive(Clone, Copy)]
struct JournalReadBounds {
    start_usn: i64,
    end_usn: i64,
    max_records: usize,
    max_evidence_bytes: usize,
}

#[derive(Clone, Debug)]
struct ResolvedRoot {
    root: IncrementalCatalogRoot,
    volume: VolumeDescriptor,
    filesystem: String,
    root_guid_path: String,
}

#[derive(Clone, Debug)]
struct VolumeDescriptor {
    volume_id: String,
    open_path: String,
}

#[derive(Clone, Copy, Debug)]
struct JournalMetadata {
    journal_id: u64,
    first_usn: i64,
    next_usn: i64,
    minimum_major_version: u16,
    maximum_major_version: u16,
}

#[derive(Clone, Debug)]
struct ResolvedJournalChange {
    full_path: String,
    file_reference: FileReference,
    usn: i64,
    observed_unix_ms: i64,
    kind: LibraryChangeObservationKind,
    rename_role: Option<RenameRole>,
    is_directory: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RenameRole {
    OldName,
    NewName,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
enum FileReference {
    V2([u8; 8]),
    V3([u8; 16]),
}

#[derive(Clone, Debug)]
struct ParsedUsnRecord {
    file_reference: FileReference,
    parent_reference: FileReference,
    usn: i64,
    observed_unix_ms: i64,
    reason: u32,
    file_attributes: u32,
    name: String,
}

#[derive(Clone, Copy)]
pub(crate) struct Win32UsnBackend;

impl UsnJournalBackend for Win32UsnBackend {
    fn describe_root(&self, root: &IncrementalCatalogRoot) -> Result<ResolvedRoot, ScanError> {
        let root_path = root.root_path.replace('/', "\\");
        let root_wide = wide_null(&root_path)?;
        let volume_mount = get_volume_path(&root_wide)?;
        let volume_name = get_volume_name(&volume_mount)?;
        let filesystem = get_filesystem_name(&volume_mount)?;
        let root_handle = open_path_handle(&root_path, FILE_READ_ATTRIBUTES)?;
        let root_guid_path = normalize_path(&final_guid_path(root_handle.raw())?);
        let open_path = volume_name.trim_end_matches('\\').to_owned();
        let volume_id = normalize_path(&volume_name).to_ascii_lowercase();
        Ok(ResolvedRoot {
            root: root.clone(),
            volume: VolumeDescriptor {
                volume_id,
                open_path,
            },
            filesystem,
            root_guid_path,
        })
    }

    fn query_journal(&self, volume: &VolumeDescriptor) -> Result<JournalMetadata, ScanError> {
        let handle = open_volume_handle(&volume.open_path)?;
        query_journal(handle.raw())
    }

    fn read_journal(
        &self,
        volume: &VolumeDescriptor,
        metadata: &JournalMetadata,
        bounds: JournalReadBounds,
        cancelled: &AtomicBool,
    ) -> Result<Vec<ResolvedJournalChange>, ScanError> {
        if metadata.maximum_major_version < 2 || metadata.minimum_major_version > 3 {
            return Err(ScanError::new(
                "usn_record_version_unsupported",
                "The volume does not expose supported V2 or V3 journal records",
            ));
        }
        let handle = open_volume_handle(&volume.open_path)?;
        let records = read_journal_records(
            handle.raw(),
            metadata.journal_id,
            bounds.start_usn,
            bounds.end_usn,
            bounds.max_records,
            bounds.max_evidence_bytes,
            cancelled,
        )?;
        resolve_record_paths(
            handle.raw(),
            &records.records,
            records.retained_bytes,
            bounds.max_evidence_bytes,
            cancelled,
        )
    }
}

fn validate_continuity(
    checkpoint: &LibraryChangeCatchUpCheckpoint,
    metadata: &JournalMetadata,
    root_set_fingerprint: &str,
    catalog_revision: u64,
) -> Result<(), ()> {
    let journal_id = checkpoint.journal_id.parse::<u64>().map_err(|_| ())?;
    let next_usn = checkpoint.next_usn.parse::<i64>().map_err(|_| ())?;
    if journal_id != metadata.journal_id
        || next_usn < metadata.first_usn
        || next_usn > metadata.next_usn
        || checkpoint.root_set_fingerprint != root_set_fingerprint
        || checkpoint.catalog_revision > catalog_revision
    {
        return Err(());
    }
    Ok(())
}

fn parse_checkpoint_usn(checkpoint: &LibraryChangeCatchUpCheckpoint) -> Result<i64, ScanError> {
    checkpoint.next_usn.parse::<i64>().map_err(|_| {
        ScanError::new(
            "usn_checkpoint_invalid",
            "The stored journal watermark is outside the supported range",
        )
    })
}

fn root_set_fingerprint(roots: &[ResolvedRoot]) -> String {
    let mut hasher = Hasher::new();
    hasher.update(b"ame-usn-root-set-v1\0");
    for root in roots {
        hasher.update(root.root.root_id.as_bytes());
        hasher.update(&[0]);
        hasher.update(root.root.root_generation.value().to_string().as_bytes());
        hasher.update(&[0]);
        hasher.update(
            normalize_path(&root.root_guid_path)
                .to_ascii_lowercase()
                .as_bytes(),
        );
        hasher.update(&[0]);
    }
    hasher.finalize().to_hex().to_string()
}

fn catch_up_evidence(
    volume: &VolumeDescriptor,
    journal_id: u64,
    next_usn: i64,
) -> LibraryChangeCatchUpEvidence {
    LibraryChangeCatchUpEvidence {
        source: CATCH_UP_SOURCE.to_owned(),
        watermark: format!("{}|{journal_id}|{next_usn}", volume.volume_id),
    }
}

fn fallback_root(
    root: &IncrementalCatalogRoot,
    code: &str,
    evidence: Option<LibraryChangeCatchUpEvidence>,
) -> LibraryChangeCatchUpRootResult {
    LibraryChangeCatchUpRootResult {
        root_id: root.root_id.clone(),
        root_generation: root.root_generation,
        observations: Vec::new(),
        fallback_code: Some(code.to_owned()),
        evidence,
    }
}

fn distribute_changes(
    roots: &[ResolvedRoot],
    mut changes: Vec<ResolvedJournalChange>,
    max_observations_per_root: usize,
    evidence: &LibraryChangeCatchUpEvidence,
) -> Result<Vec<LibraryChangeCatchUpRootResult>, ScanError> {
    changes.sort_by_key(|change| change.usn);
    let mut pending_old_names = BTreeMap::<FileReference, usize>::new();
    let mut paired_old_names = BTreeSet::new();
    let mut rename_pairs = BTreeMap::<usize, usize>::new();
    for (index, change) in changes.iter().enumerate() {
        match change.rename_role {
            Some(RenameRole::OldName) => {
                pending_old_names.insert(change.file_reference, index);
            }
            Some(RenameRole::NewName) => {
                if let Some(previous_index) = pending_old_names.remove(&change.file_reference) {
                    paired_old_names.insert(previous_index);
                    rename_pairs.insert(index, previous_index);
                }
            }
            None => {}
        }
    }
    let mut observations = roots
        .iter()
        .map(|root| (root.root.root_id.as_str(), BTreeMap::new()))
        .collect::<BTreeMap<_, BTreeMap<(LibraryChangeScope, String), LibraryChangeObservation>>>();
    for (index, change) in changes.iter().enumerate() {
        if paired_old_names.contains(&index) {
            continue;
        }
        for root in roots {
            let current_path =
                relative_to_root(&normalize_path(&change.full_path), &root.root_guid_path);
            let paired_previous = rename_pairs
                .get(&index)
                .and_then(|previous_index| changes.get(*previous_index));
            let previous_path = paired_previous.and_then(|previous| {
                relative_to_root(&normalize_path(&previous.full_path), &root.root_guid_path)
            });
            let (observation_change, relative_path, previous_relative_path, kind) =
                match (paired_previous, previous_path, current_path) {
                    (Some(_), Some(previous_path), Some(current_path)) => (
                        change,
                        current_path,
                        Some(previous_path),
                        LibraryChangeObservationKind::Renamed {
                            is_reliably_paired: true,
                        },
                    ),
                    (Some(previous), Some(previous_path), None) => (
                        previous,
                        previous_path,
                        None,
                        LibraryChangeObservationKind::Removed,
                    ),
                    (Some(_), None, Some(current_path)) => (
                        change,
                        current_path,
                        None,
                        LibraryChangeObservationKind::Created,
                    ),
                    (Some(_), None, None) | (None, _, None) => continue,
                    (None, _, Some(current_path)) => (change, current_path, None, change.kind),
                };
            if relative_path.is_empty() && !observation_change.is_directory {
                return Err(ScanError::new(
                    "usn_path_reconstruction_invalid",
                    "A file journal record resolved to the root directory itself",
                ));
            }
            let scope = if observation_change.is_directory {
                if relative_path.is_empty() {
                    LibraryChangeScope::Root
                } else {
                    LibraryChangeScope::Subtree
                }
            } else {
                LibraryChangeScope::Path
            };
            let key = (scope, relative_path.to_ascii_lowercase());
            let observation = LibraryChangeObservation {
                root_id: root.root.root_id.clone(),
                root_generation: root.root.root_generation,
                sequence: u64::try_from(change.usn).map_err(|_| {
                    ScanError::new(
                        "usn_sequence_invalid",
                        "A journal record contained a negative update sequence number",
                    )
                })?,
                observed_unix_ms: change.observed_unix_ms,
                kind: if observation_change.is_directory
                    && !matches!(kind, LibraryChangeObservationKind::Renamed { .. })
                {
                    LibraryChangeObservationKind::DirectoryChanged
                } else {
                    kind
                },
                scope,
                relative_path,
                previous_relative_path,
                origin: LibraryChangeOrigin::StartupCatchUp,
            };
            let root_observations = observations
                .get_mut(root.root.root_id.as_str())
                .ok_or_else(|| {
                    ScanError::new(
                        "usn_root_distribution_invalid",
                        "A resolved journal root lost its bounded observation state",
                    )
                })?;
            merge_observation(root_observations, key, observation)?;
            if root_observations.len() > max_observations_per_root {
                return Err(ScanError::new(
                    "usn_candidate_limit_exceeded",
                    "One root exceeded the bounded journal candidate limit",
                ));
            }
        }
    }
    Ok(roots
        .iter()
        .map(|root| LibraryChangeCatchUpRootResult {
            root_id: root.root.root_id.clone(),
            root_generation: root.root.root_generation,
            observations: observations
                .remove(root.root.root_id.as_str())
                .unwrap_or_default()
                .into_values()
                .collect(),
            fallback_code: None,
            evidence: Some(evidence.clone()),
        })
        .collect())
}

fn merge_observation(
    observations: &mut BTreeMap<(LibraryChangeScope, String), LibraryChangeObservation>,
    key: (LibraryChangeScope, String),
    observation: LibraryChangeObservation,
) -> Result<(), ScanError> {
    let Some(existing) = observations.get_mut(&key) else {
        observations.insert(key, observation);
        return Ok(());
    };
    if existing.sequence > observation.sequence {
        return Ok(());
    }
    match (existing.kind, observation.kind) {
        (
            LibraryChangeObservationKind::Renamed { .. },
            LibraryChangeObservationKind::Renamed { .. },
        ) if existing.previous_relative_path != observation.previous_relative_path => {
            return Err(ScanError::new(
                "usn_rename_conflict",
                "Multiple bounded journal renames targeted the same path",
            ));
        }
        (LibraryChangeObservationKind::Renamed { .. }, _) => {
            existing.sequence = observation.sequence;
            existing.observed_unix_ms = observation.observed_unix_ms;
        }
        _ => {
            *existing = observation;
        }
    }
    Ok(())
}

fn relative_to_root(full_path: &str, root_path: &str) -> Option<String> {
    let full = normalize_path(full_path);
    let root = normalize_path(root_path);
    if full.eq_ignore_ascii_case(&root) {
        return Some(String::new());
    }
    let prefix = format!("{root}/");
    if full.len() <= prefix.len()
        || !full
            .get(..prefix.len())
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(&prefix))
    {
        return None;
    }
    full.get(prefix.len()..).map(str::to_owned)
}

fn filesystem_supports_usn(filesystem: &str) -> bool {
    filesystem.eq_ignore_ascii_case("NTFS") || filesystem.eq_ignore_ascii_case("ReFS")
}

fn resolved_changes_bytes(changes: &[ResolvedJournalChange]) -> Option<usize> {
    changes.iter().try_fold(0_usize, |total, change| {
        total
            .checked_add(size_of::<ResolvedJournalChange>())
            .and_then(|value| value.checked_add(change.full_path.len()))
    })
}

fn read_journal_records(
    handle: HANDLE,
    journal_id: u64,
    mut start_usn: i64,
    end_usn: i64,
    max_records: usize,
    max_evidence_bytes: usize,
    cancelled: &AtomicBool,
) -> Result<ParsedJournalRecords, ScanError> {
    let mut records = Vec::new();
    let mut retained_bytes = 0_usize;
    while start_usn < end_usn {
        if cancelled.load(Ordering::Acquire) {
            return Err(catch_up_cancelled());
        }
        let request = READ_USN_JOURNAL_DATA_V1 {
            StartUsn: start_usn,
            ReasonMask: u32::MAX,
            ReturnOnlyOnClose: 0,
            Timeout: 0,
            BytesToWaitFor: 0,
            UsnJournalID: journal_id,
            MinMajorVersion: 2,
            MaxMajorVersion: 3,
        };
        let mut output = vec![0_u8; JOURNAL_BUFFER_BYTES];
        let bytes = device_io_control(
            handle,
            FSCTL_READ_USN_JOURNAL,
            (&raw const request).cast(),
            size_of::<READ_USN_JOURNAL_DATA_V1>(),
            output.as_mut_ptr().cast(),
            output.len(),
            "usn_journal_read_failed",
        )?;
        output.truncate(bytes);
        let (next_usn, mut batch) = parse_journal_buffer(&output)?;
        batch.retain(|record| record.usn < end_usn);
        if records.len().saturating_add(batch.len()) > max_records {
            return Err(ScanError::new(
                "usn_record_limit_exceeded",
                "One volume exceeded the bounded journal record limit",
            ));
        }
        let batch_bytes = batch.iter().try_fold(0_usize, |total, record| {
            total
                .checked_add(size_of::<ParsedUsnRecord>())
                .and_then(|value| value.checked_add(record.name.len()))
                .ok_or_else(evidence_bytes_limit_exceeded)
        })?;
        retained_bytes = retained_bytes
            .checked_add(batch_bytes)
            .filter(|total| *total <= max_evidence_bytes)
            .ok_or_else(evidence_bytes_limit_exceeded)?;
        records.append(&mut batch);
        let advanced = next_usn.min(end_usn);
        if advanced <= start_usn {
            return Err(ScanError::new(
                "usn_journal_did_not_advance",
                "The journal reader did not advance its exclusive watermark",
            ));
        }
        start_usn = advanced;
    }
    Ok(ParsedJournalRecords {
        records,
        retained_bytes,
    })
}

struct ParsedJournalRecords {
    records: Vec<ParsedUsnRecord>,
    retained_bytes: usize,
}

fn parse_journal_buffer(buffer: &[u8]) -> Result<(i64, Vec<ParsedUsnRecord>), ScanError> {
    if buffer.len() < size_of::<i64>() {
        return Err(invalid_record_buffer());
    }
    let next_usn = i64::from_le_bytes(
        buffer[..8]
            .try_into()
            .map_err(|_| invalid_record_buffer())?,
    );
    let mut offset = 8_usize;
    let mut records = Vec::new();
    while offset < buffer.len() {
        let remaining = buffer.get(offset..).ok_or_else(invalid_record_buffer)?;
        if remaining.len() < 8 {
            return Err(invalid_record_buffer());
        }
        let record_length =
            usize::try_from(read_u32(remaining, 0)?).map_err(|_| invalid_record())?;
        if record_length < 60 || !record_length.is_multiple_of(8) || record_length > remaining.len()
        {
            return Err(invalid_record());
        }
        let record = remaining.get(..record_length).ok_or_else(invalid_record)?;
        records.push(parse_usn_record(record)?);
        offset = offset
            .checked_add(record_length)
            .ok_or_else(invalid_record)?;
    }
    Ok((next_usn, records))
}

fn parse_usn_record(record: &[u8]) -> Result<ParsedUsnRecord, ScanError> {
    let major_version = read_u16(record, 4)?;
    let (
        file_reference,
        parent_reference,
        usn_offset,
        timestamp_offset,
        reason_offset,
        attributes_offset,
        filename_length_offset,
        filename_offset_offset,
        minimum_length,
    ) = match major_version {
        2 => (
            FileReference::V2(read_array::<8>(record, 8)?),
            FileReference::V2(read_array::<8>(record, 16)?),
            24,
            32,
            40,
            52,
            56,
            58,
            60,
        ),
        3 => (
            FileReference::V3(read_array::<16>(record, 8)?),
            FileReference::V3(read_array::<16>(record, 24)?),
            40,
            48,
            56,
            68,
            72,
            74,
            76,
        ),
        _ => {
            return Err(ScanError::new(
                "usn_record_version_unsupported",
                "The journal returned a record version outside V2 and V3",
            ));
        }
    };
    if record.len() < minimum_length {
        return Err(invalid_record());
    }
    let filename_length = usize::from(read_u16(record, filename_length_offset)?);
    let filename_offset = usize::from(read_u16(record, filename_offset_offset)?);
    if !filename_length.is_multiple_of(2)
        || !filename_offset.is_multiple_of(2)
        || filename_offset < minimum_length
    {
        return Err(invalid_record());
    }
    let filename_end = filename_offset
        .checked_add(filename_length)
        .ok_or_else(invalid_record)?;
    let filename_bytes = record
        .get(filename_offset..filename_end)
        .ok_or_else(invalid_record)?;
    let mut filename_utf16 = Vec::with_capacity(filename_length / 2);
    for bytes in filename_bytes.chunks_exact(2) {
        filename_utf16.push(u16::from_le_bytes([bytes[0], bytes[1]]));
    }
    let name = String::from_utf16(&filename_utf16).map_err(|_| {
        ScanError::new(
            "usn_filename_invalid",
            "The journal returned an invalid UTF-16 file name",
        )
    })?;
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['\\', '/'])
        || name.contains('\0')
    {
        return Err(invalid_record());
    }
    let timestamp = read_i64(record, timestamp_offset)?;
    Ok(ParsedUsnRecord {
        file_reference,
        parent_reference,
        usn: read_i64(record, usn_offset)?,
        observed_unix_ms: timestamp
            .saturating_sub(WINDOWS_TO_UNIX_EPOCH_100NS)
            .checked_div(HUNDRED_NS_PER_MILLISECOND)
            .unwrap_or(0)
            .max(0),
        reason: read_u32(record, reason_offset)?,
        file_attributes: read_u32(record, attributes_offset)?,
        name,
    })
}

fn resolve_record_paths(
    volume_handle: HANDLE,
    records: &[ParsedUsnRecord],
    mut retained_bytes: usize,
    max_evidence_bytes: usize,
    cancelled: &AtomicBool,
) -> Result<Vec<ResolvedJournalChange>, ScanError> {
    let mut histories = BTreeMap::<FileReference, Vec<usize>>::new();
    for (index, record) in records.iter().enumerate() {
        histories
            .entry(record.file_reference)
            .or_default()
            .push(index);
    }
    let mut resolved = Vec::with_capacity(records.len());
    for record in records {
        if cancelled.load(Ordering::Acquire) {
            return Err(catch_up_cancelled());
        }
        let rename_role = rename_role(record.reason)?;
        let mut visiting = BTreeSet::new();
        let parent = resolve_reference_path(
            volume_handle,
            record.parent_reference,
            record.usn,
            records,
            &histories,
            &mut visiting,
        )?;
        let full_path = format!("{}/{}", normalize_path(&parent), record.name);
        retained_bytes = retained_bytes
            .checked_add(size_of::<ResolvedJournalChange>())
            .and_then(|value| value.checked_add(full_path.len()))
            .filter(|total| *total <= max_evidence_bytes)
            .ok_or_else(evidence_bytes_limit_exceeded)?;
        resolved.push(ResolvedJournalChange {
            full_path,
            file_reference: record.file_reference,
            usn: record.usn,
            observed_unix_ms: record.observed_unix_ms,
            kind: observation_kind(record.reason),
            rename_role,
            is_directory: record.file_attributes & FILE_ATTRIBUTE_DIRECTORY != 0,
        });
    }
    Ok(resolved)
}

fn resolve_reference_path(
    volume_handle: HANDLE,
    reference: FileReference,
    before_usn: i64,
    records: &[ParsedUsnRecord],
    histories: &BTreeMap<FileReference, Vec<usize>>,
    visiting: &mut BTreeSet<FileReference>,
) -> Result<String, ScanError> {
    if !visiting.insert(reference) || visiting.len() > 256 {
        return Err(ScanError::new(
            "usn_path_reconstruction_cycle",
            "The journal parent chain could not be reconstructed safely",
        ));
    }
    let historical = histories.get(&reference).and_then(|indices| {
        indices
            .iter()
            .rev()
            .filter_map(|index| records.get(*index))
            .find(|record| record.usn < before_usn)
    });
    let result = if let Some(parent_record) = historical {
        let parent = resolve_reference_path(
            volume_handle,
            parent_record.parent_reference,
            parent_record.usn,
            records,
            histories,
            visiting,
        )?;
        Ok(format!(
            "{}/{}",
            normalize_path(&parent),
            parent_record.name
        ))
    } else {
        open_file_reference_path(volume_handle, reference)
    };
    visiting.remove(&reference);
    result
}

fn open_file_reference_path(
    volume_handle: HANDLE,
    reference: FileReference,
) -> Result<String, ScanError> {
    let descriptor = match reference {
        FileReference::V2(bytes) => FILE_ID_DESCRIPTOR {
            dwSize: u32::try_from(size_of::<FILE_ID_DESCRIPTOR>()).unwrap_or(u32::MAX),
            Type: FileIdType,
            Anonymous: FILE_ID_DESCRIPTOR_0 {
                FileId: i64::from_le_bytes(bytes),
            },
        },
        FileReference::V3(bytes) => FILE_ID_DESCRIPTOR {
            dwSize: u32::try_from(size_of::<FILE_ID_DESCRIPTOR>()).unwrap_or(u32::MAX),
            Type: ExtendedFileIdType,
            Anonymous: FILE_ID_DESCRIPTOR_0 {
                ExtendedFileId: FILE_ID_128 { Identifier: bytes },
            },
        },
    };
    // SAFETY: ADR 0022 defines this boundary. The descriptor is fully initialized for its exact
    // identifier kind, the volume handle is borrowed for this call, and no pointer is retained.
    let raw = unsafe {
        OpenFileById(
            volume_handle,
            &raw const descriptor,
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            FILE_FLAG_BACKUP_SEMANTICS,
        )
    };
    let handle = OwnedHandle::new(raw).ok_or_else(|| {
        windows_error(
            "usn_path_reconstruction_failed",
            "Could not open a journal parent by file identifier",
        )
    })?;
    final_guid_path(handle.raw())
}

fn query_journal(handle: HANDLE) -> Result<JournalMetadata, ScanError> {
    let mut output = USN_JOURNAL_DATA_V2::default();
    let bytes = device_io_control(
        handle,
        FSCTL_QUERY_USN_JOURNAL,
        null(),
        0,
        (&raw mut output).cast(),
        size_of::<USN_JOURNAL_DATA_V2>(),
        "usn_journal_query_failed",
    )?;
    if bytes < size_of::<windows_sys::Win32::System::Ioctl::USN_JOURNAL_DATA_V0>()
        || output.FirstUsn < 0
        || output.NextUsn < output.FirstUsn
    {
        return Err(ScanError::new(
            "usn_journal_metadata_invalid",
            "The volume returned invalid journal metadata",
        ));
    }
    let has_versions = bytes >= size_of::<windows_sys::Win32::System::Ioctl::USN_JOURNAL_DATA_V1>();
    Ok(JournalMetadata {
        journal_id: output.UsnJournalID,
        first_usn: output.FirstUsn,
        next_usn: output.NextUsn,
        minimum_major_version: if has_versions {
            output.MinSupportedMajorVersion
        } else {
            2
        },
        maximum_major_version: if has_versions {
            output.MaxSupportedMajorVersion
        } else {
            2
        },
    })
}

fn device_io_control(
    handle: HANDLE,
    control_code: u32,
    input: *const c_void,
    input_bytes: usize,
    output: *mut c_void,
    output_bytes: usize,
    error_code: &str,
) -> Result<usize, ScanError> {
    let input_bytes = u32::try_from(input_bytes).map_err(|_| invalid_record_buffer())?;
    let output_bytes = u32::try_from(output_bytes).map_err(|_| invalid_record_buffer())?;
    let mut returned = 0_u32;
    // SAFETY: ADR 0022 defines this boundary. Buffers remain alive for the synchronous call, their
    // sizes fit u32, the returned count is checked, and no overlapped pointer or buffer escapes.
    let succeeded = unsafe {
        DeviceIoControl(
            handle,
            control_code,
            input,
            input_bytes,
            output,
            output_bytes,
            &raw mut returned,
            null_mut(),
        )
    };
    if succeeded == 0 {
        return Err(windows_error(
            error_code,
            "The Windows change journal operation failed",
        ));
    }
    if returned > output_bytes {
        return Err(invalid_record_buffer());
    }
    Ok(returned as usize)
}

fn open_volume_handle(path: &str) -> Result<OwnedHandle, ScanError> {
    open_path_handle(path, GENERIC_READ)
}

fn open_path_handle(path: &str, desired_access: u32) -> Result<OwnedHandle, ScanError> {
    let wide = wide_null(path)?;
    // SAFETY: ADR 0022 defines this boundary. The UTF-16 path is NUL-terminated and lives through
    // the call; null security/template pointers are allowed and no pointer is retained.
    let raw = unsafe {
        CreateFileW(
            wide.as_ptr(),
            desired_access,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            null_mut(),
        )
    };
    OwnedHandle::new(raw).ok_or_else(|| {
        windows_error(
            "usn_volume_open_failed",
            "Could not open the Windows volume or library root",
        )
    })
}

fn final_guid_path(handle: HANDLE) -> Result<String, ScanError> {
    let mut capacity = 512_usize;
    loop {
        if capacity > MAX_PATH_UTF16 {
            return Err(ScanError::new(
                "usn_path_limit_exceeded",
                "A reconstructed Windows path exceeded the supported bound",
            ));
        }
        let mut buffer = vec![0_u16; capacity];
        // SAFETY: ADR 0022 defines this boundary. The initialized UTF-16 buffer remains alive for
        // the call, its capacity fits u32, and the returned length is checked before conversion.
        let length = unsafe {
            GetFinalPathNameByHandleW(
                handle,
                buffer.as_mut_ptr(),
                u32::try_from(buffer.len()).unwrap_or(u32::MAX),
                VOLUME_NAME_GUID,
            )
        };
        if length == 0 {
            return Err(windows_error(
                "usn_path_reconstruction_failed",
                "Could not resolve a file identifier to a volume path",
            ));
        }
        let length = usize::try_from(length).map_err(|_| invalid_record_buffer())?;
        if length < buffer.len() {
            buffer.truncate(length);
            return String::from_utf16(&buffer).map_err(|_| {
                ScanError::new(
                    "usn_path_encoding_invalid",
                    "Windows returned an invalid UTF-16 path",
                )
            });
        }
        capacity = length.saturating_add(1);
    }
}

fn get_volume_path(root: &[u16]) -> Result<Vec<u16>, ScanError> {
    let mut output = vec![0_u16; MAX_PATH_UTF16];
    // SAFETY: ADR 0022 defines this boundary. Input and output are NUL-terminated/live buffers and
    // the fixed output capacity fits u32; Win32 retains neither pointer.
    let succeeded = unsafe {
        GetVolumePathNameW(
            root.as_ptr(),
            output.as_mut_ptr(),
            u32::try_from(output.len()).unwrap_or(u32::MAX),
        )
    };
    if succeeded == 0 {
        return Err(windows_error(
            "usn_volume_resolution_failed",
            "Could not resolve the library root volume",
        ));
    }
    truncate_at_nul(&mut output)?;
    output.push(0);
    Ok(output)
}

fn get_volume_name(mount_path: &[u16]) -> Result<String, ScanError> {
    let mut output = vec![0_u16; 512];
    // SAFETY: ADR 0022 defines this boundary. Both buffers are valid for the call, capacity fits
    // u32, and the result is NUL-checked before conversion.
    let succeeded = unsafe {
        GetVolumeNameForVolumeMountPointW(
            mount_path.as_ptr(),
            output.as_mut_ptr(),
            u32::try_from(output.len()).unwrap_or(u32::MAX),
        )
    };
    if succeeded == 0 {
        return Err(windows_error(
            "usn_volume_resolution_failed",
            "Could not resolve the stable Windows volume name",
        ));
    }
    wide_output_to_string(output)
}

fn get_filesystem_name(mount_path: &[u16]) -> Result<String, ScanError> {
    let mut filesystem = vec![0_u16; 64];
    // SAFETY: ADR 0022 defines this boundary. The NUL-terminated mount path and initialized output
    // buffer live for the complete call, optional outputs are null, and no pointer is retained.
    let succeeded = unsafe {
        GetVolumeInformationW(
            mount_path.as_ptr(),
            null_mut(),
            0,
            null_mut(),
            null_mut(),
            null_mut(),
            filesystem.as_mut_ptr(),
            u32::try_from(filesystem.len()).unwrap_or(u32::MAX),
        )
    };
    if succeeded == 0 {
        return Err(windows_error(
            "usn_filesystem_query_failed",
            "Could not determine the library root filesystem",
        ));
    }
    wide_output_to_string(filesystem)
}

fn wide_null(value: &str) -> Result<Vec<u16>, ScanError> {
    if value.contains('\0') {
        return Err(ScanError::new(
            "usn_path_invalid",
            "A Windows path cannot contain an embedded NUL",
        ));
    }
    let mut wide = OsStr::new(value).encode_wide().collect::<Vec<_>>();
    if wide.len() >= MAX_PATH_UTF16 {
        return Err(ScanError::new(
            "usn_path_limit_exceeded",
            "A Windows path exceeded the supported bound",
        ));
    }
    wide.push(0);
    Ok(wide)
}

fn wide_output_to_string(mut value: Vec<u16>) -> Result<String, ScanError> {
    truncate_at_nul(&mut value)?;
    String::from_utf16(&value).map_err(|_| {
        ScanError::new(
            "usn_path_encoding_invalid",
            "Windows returned invalid UTF-16 volume data",
        )
    })
}

fn truncate_at_nul(value: &mut Vec<u16>) -> Result<(), ScanError> {
    let length = value.iter().position(|unit| *unit == 0).ok_or_else(|| {
        ScanError::new(
            "usn_path_truncated",
            "Windows returned a path without a terminating NUL",
        )
    })?;
    value.truncate(length);
    Ok(())
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_owned()
}

fn observation_kind(reason: u32) -> LibraryChangeObservationKind {
    if reason & USN_REASON_RENAME_OLD_NAME != 0 || reason & USN_REASON_FILE_DELETE != 0 {
        LibraryChangeObservationKind::Removed
    } else if reason & USN_REASON_RENAME_NEW_NAME != 0 || reason & USN_REASON_FILE_CREATE != 0 {
        LibraryChangeObservationKind::Created
    } else {
        LibraryChangeObservationKind::Modified
    }
}

fn rename_role(reason: u32) -> Result<Option<RenameRole>, ScanError> {
    let has_old_name = reason & USN_REASON_RENAME_OLD_NAME != 0;
    let has_new_name = reason & USN_REASON_RENAME_NEW_NAME != 0;
    match (has_old_name, has_new_name) {
        (true, false) => Ok(Some(RenameRole::OldName)),
        (false, true) => Ok(Some(RenameRole::NewName)),
        (false, false) => Ok(None),
        (true, true) => Err(ScanError::new(
            "usn_rename_pair_ambiguous",
            "A journal record combined old-name and new-name rename evidence",
        )),
    }
}

fn read_array<const N: usize>(buffer: &[u8], offset: usize) -> Result<[u8; N], ScanError> {
    buffer
        .get(offset..offset.saturating_add(N))
        .and_then(|value| value.try_into().ok())
        .ok_or_else(invalid_record)
}

fn read_u16(buffer: &[u8], offset: usize) -> Result<u16, ScanError> {
    Ok(u16::from_le_bytes(read_array(buffer, offset)?))
}

fn read_u32(buffer: &[u8], offset: usize) -> Result<u32, ScanError> {
    Ok(u32::from_le_bytes(read_array(buffer, offset)?))
}

fn read_i64(buffer: &[u8], offset: usize) -> Result<i64, ScanError> {
    Ok(i64::from_le_bytes(read_array(buffer, offset)?))
}

fn invalid_record() -> ScanError {
    ScanError::new(
        "usn_record_invalid",
        "The journal returned a malformed variable-length record",
    )
}

fn invalid_record_buffer() -> ScanError {
    ScanError::new(
        "usn_record_buffer_invalid",
        "The journal returned a malformed or oversized output buffer",
    )
}

fn evidence_bytes_limit_exceeded() -> ScanError {
    ScanError::new(
        "usn_evidence_bytes_limit_exceeded",
        "One volume exceeded the bounded in-memory journal evidence budget",
    )
}

fn catch_up_cancelled() -> ScanError {
    ScanError::new(
        "library_change_catch_up_cancelled",
        "Windows journal catch-up was cancelled",
    )
}

fn windows_error(code: &str, context: &str) -> ScanError {
    ScanError::new(
        code,
        format!("{context}: {}", std::io::Error::last_os_error()),
    )
}

struct OwnedHandle(HANDLE);

impl OwnedHandle {
    fn new(raw: HANDLE) -> Option<Self> {
        if raw.is_null() || raw == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Self(raw))
        }
    }

    const fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: ADR 0022 defines this boundary. OwnedHandle is created only from a valid unique
        // Win32 handle and Drop runs exactly once; the raw handle is never closed elsewhere.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::Mutex;

    use tempfile::tempdir;

    use super::*;
    use crate::domain::LibraryRootGeneration;

    #[test]
    fn parser_accepts_v2_and_v3_records() {
        let mut buffer = 80_i64.to_le_bytes().to_vec();
        buffer.extend(v2_record(40, 8, "旧图.jpg", USN_REASON_FILE_DELETE));
        buffer.extend(v3_record(48, 16, "新图.jpg", USN_REASON_FILE_CREATE));

        let (next, records) = parse_journal_buffer(&buffer).expect("valid mixed records");

        assert_eq!(next, 80);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].name, "旧图.jpg");
        assert!(matches!(records[1].file_reference, FileReference::V3(_)));
    }

    #[test]
    fn parser_rejects_filename_outside_record() {
        let mut record = v2_record(40, 8, "photo.jpg", USN_REASON_FILE_CREATE);
        record[58..60].copy_from_slice(&u16::MAX.to_le_bytes());
        let mut buffer = 50_i64.to_le_bytes().to_vec();
        buffer.extend(record);

        let error = parse_journal_buffer(&buffer).expect_err("invalid filename offset");

        assert_eq!(error.code, "usn_record_invalid");
    }

    #[test]
    fn parser_rejects_invalid_utf16_filename() {
        let mut record = v2_record(40, 8, "photo.jpg", USN_REASON_FILE_CREATE);
        record[60..62].copy_from_slice(&0xD800_u16.to_le_bytes());
        record[56..58].copy_from_slice(&2_u16.to_le_bytes());
        let mut buffer = 50_i64.to_le_bytes().to_vec();
        buffer.extend(record);

        let error = parse_journal_buffer(&buffer).expect_err("invalid UTF-16 filename");

        assert_eq!(error.code, "usn_filename_invalid");
    }

    #[test]
    fn parser_rejects_unsupported_record_version() {
        let mut record = v2_record(40, 8, "photo.jpg", USN_REASON_FILE_CREATE);
        record[4..6].copy_from_slice(&4_u16.to_le_bytes());
        let mut buffer = 50_i64.to_le_bytes().to_vec();
        buffer.extend(record);

        let error = parse_journal_buffer(&buffer).expect_err("unsupported record version");

        assert_eq!(error.code, "usn_record_version_unsupported");
    }

    #[test]
    fn parser_rejects_unaligned_record_length() {
        let mut record = v2_record(40, 8, "photo.jpg", USN_REASON_FILE_CREATE);
        record[..4].copy_from_slice(&61_u32.to_le_bytes());
        let mut buffer = 50_i64.to_le_bytes().to_vec();
        buffer.extend(record);

        let error = parse_journal_buffer(&buffer).expect_err("unaligned record length");

        assert_eq!(error.code, "usn_record_invalid");
    }

    #[test]
    fn one_volume_read_is_shared_by_multiple_roots() {
        let backend = FakeBackend::new();
        let source = WindowsUsnCatchUpSource {
            backend: backend.clone(),
        };
        let roots = vec![
            root("root-a", "C:/library-a"),
            root("root-b", "C:/library-b"),
        ];
        let fingerprint = root_set_fingerprint(&[
            backend.describe_root(&roots[0]).expect("root a"),
            backend.describe_root(&roots[1]).expect("root b"),
        ]);
        let checkpoints = vec![LibraryChangeCatchUpCheckpoint {
            volume_id: "//?/volume{fixture}".to_owned(),
            journal_id: "12".to_owned(),
            next_usn: "20".to_owned(),
            root_set_fingerprint: fingerprint,
            catalog_revision: 7,
            updated_unix_ms: 1,
        }];

        let batch = source
            .read_changes(
                &roots,
                &checkpoints,
                50,
                Default::default(),
                &AtomicBool::new(false),
            )
            .expect("shared catch up");

        assert_eq!(backend.read_count(), 1);
        assert_eq!(batch.roots.len(), 2);
        assert!(batch.roots.iter().all(|root| root.fallback_code.is_none()));
        assert!(batch.roots.iter().all(|root| root.observations.len() == 1));
    }

    #[test]
    fn journal_identity_mismatch_falls_back_and_rebaselines() {
        let backend = FakeBackend::new();
        let source = WindowsUsnCatchUpSource {
            backend: backend.clone(),
        };
        let root = root("root-a", "C:/library-a");
        let resolved = backend.describe_root(&root).expect("resolved root");
        let checkpoint = checkpoint_for(&[resolved], "11", 7);

        let batch = source
            .read_changes(
                &[root],
                &[checkpoint],
                50,
                Default::default(),
                &AtomicBool::new(false),
            )
            .expect("fallback");

        assert_eq!(backend.read_count(), 0);
        assert_eq!(
            batch.roots[0].fallback_code.as_deref(),
            Some("usn_continuity_invalid")
        );
        assert_eq!(batch.checkpoints[0].journal_id, "12");
        assert_eq!(batch.checkpoints[0].next_usn, "40");
    }

    #[test]
    fn root_set_mismatch_falls_back_without_reading_the_journal() {
        let backend = FakeBackend::new();
        let source = WindowsUsnCatchUpSource {
            backend: backend.clone(),
        };
        let root = root("root-a", "C:/library-a");
        let checkpoint = LibraryChangeCatchUpCheckpoint {
            root_set_fingerprint: "a".repeat(64),
            ..checkpoint_for(
                &[backend.describe_root(&root).expect("resolved root")],
                "12",
                7,
            )
        };

        let batch = source
            .read_changes(
                &[root],
                &[checkpoint],
                50,
                Default::default(),
                &AtomicBool::new(false),
            )
            .expect("root-set fallback");

        assert_eq!(backend.read_count(), 0);
        assert_eq!(
            batch.roots[0].fallback_code.as_deref(),
            Some("usn_continuity_invalid")
        );
    }

    #[test]
    fn future_catalog_revision_falls_back_without_reading_the_journal() {
        let backend = FakeBackend::new();
        let source = WindowsUsnCatchUpSource {
            backend: backend.clone(),
        };
        let root = root("root-a", "C:/library-a");
        let checkpoint = checkpoint_for(
            &[backend.describe_root(&root).expect("resolved root")],
            "12",
            8,
        );

        let batch = source
            .read_changes(
                &[root],
                &[checkpoint],
                50,
                Default::default(),
                &AtomicBool::new(false),
            )
            .expect("catalog revision fallback");

        assert_eq!(backend.read_count(), 0);
        assert_eq!(
            batch.roots[0].fallback_code.as_deref(),
            Some("usn_continuity_invalid")
        );
    }

    #[test]
    fn candidate_capacity_overflow_falls_back_for_the_root() {
        let backend = FakeBackend::with_changes(vec![
            journal_change("//?/Volume{fixture}/library-a/one.jpg", 30),
            journal_change("//?/Volume{fixture}/library-a/two.jpg", 31),
        ]);
        let source = WindowsUsnCatchUpSource {
            backend: backend.clone(),
        };
        let root = root("root-a", "C:/library-a");
        let checkpoint = checkpoint_for(
            &[backend.describe_root(&root).expect("resolved root")],
            "12",
            7,
        );

        let batch = source
            .read_changes(
                &[root],
                &[checkpoint],
                50,
                LibraryChangeCatchUpLimits {
                    max_records_per_volume: 8,
                    max_evidence_bytes_per_volume:
                        LibraryChangeCatchUpLimits::MAX_EVIDENCE_BYTES_PER_VOLUME,
                    max_observations_per_root: 1,
                },
                &AtomicBool::new(false),
            )
            .expect("capacity fallback");

        assert_eq!(backend.read_count(), 1);
        assert_eq!(
            batch.roots[0].fallback_code.as_deref(),
            Some("usn_candidate_limit_exceeded")
        );
        assert!(batch.roots[0].observations.is_empty());
    }

    #[test]
    fn record_capacity_overflow_falls_back_for_the_volume() {
        let backend = FakeBackend::with_changes(vec![
            journal_change("//?/Volume{fixture}/library-a/one.jpg", 30),
            journal_change("//?/Volume{fixture}/library-a/two.jpg", 31),
        ]);
        let source = WindowsUsnCatchUpSource {
            backend: backend.clone(),
        };
        let root = root("root-a", "C:/library-a");
        let checkpoint = checkpoint_for(
            &[backend.describe_root(&root).expect("resolved root")],
            "12",
            7,
        );

        let batch = source
            .read_changes(
                &[root],
                &[checkpoint],
                50,
                LibraryChangeCatchUpLimits {
                    max_records_per_volume: 1,
                    max_evidence_bytes_per_volume:
                        LibraryChangeCatchUpLimits::MAX_EVIDENCE_BYTES_PER_VOLUME,
                    max_observations_per_root: 8,
                },
                &AtomicBool::new(false),
            )
            .expect("record capacity fallback");

        assert_eq!(backend.read_count(), 1);
        assert_eq!(
            batch.roots[0].fallback_code.as_deref(),
            Some("usn_record_limit_exceeded")
        );
    }

    #[test]
    fn evidence_byte_capacity_overflow_falls_back_for_the_volume() {
        let backend = FakeBackend::with_changes(vec![journal_change(
            &format!("//?/Volume{{fixture}}/library-a/{}.jpg", "a".repeat(256)),
            30,
        )]);
        let source = WindowsUsnCatchUpSource {
            backend: backend.clone(),
        };
        let root = root("root-a", "C:/library-a");
        let checkpoint = checkpoint_for(
            &[backend.describe_root(&root).expect("resolved root")],
            "12",
            7,
        );

        let batch = source
            .read_changes(
                &[root],
                &[checkpoint],
                50,
                LibraryChangeCatchUpLimits {
                    max_records_per_volume: 8,
                    max_evidence_bytes_per_volume: 128,
                    max_observations_per_root: 8,
                },
                &AtomicBool::new(false),
            )
            .expect("evidence byte fallback");

        assert_eq!(backend.read_count(), 1);
        assert_eq!(
            batch.roots[0].fallback_code.as_deref(),
            Some("usn_evidence_bytes_limit_exceeded")
        );
        assert!(batch.roots[0].observations.is_empty());
    }

    #[test]
    fn downtime_changes_become_root_relative_reconciliation_candidates() {
        let backend = FakeBackend::with_changes(vec![
            journal_change_with_kind(
                "//?/Volume{fixture}/library-a/created.jpg",
                30,
                LibraryChangeObservationKind::Created,
            ),
            journal_rename_change(
                "//?/Volume{fixture}/library-a/old.jpg",
                31,
                7,
                RenameRole::OldName,
            ),
            journal_rename_change(
                "//?/Volume{fixture}/library-a/new.jpg",
                32,
                7,
                RenameRole::NewName,
            ),
            journal_change_with_kind(
                "//?/Volume{fixture}/library-a/new.jpg",
                33,
                LibraryChangeObservationKind::Modified,
            ),
            journal_change_with_kind(
                "//?/Volume{fixture}/library-a/removed.jpg",
                34,
                LibraryChangeObservationKind::Removed,
            ),
        ]);
        let source = WindowsUsnCatchUpSource {
            backend: backend.clone(),
        };
        let root = root("root-a", "C:/library-a");
        let checkpoint = checkpoint_for(
            &[backend.describe_root(&root).expect("resolved root")],
            "12",
            7,
        );

        let batch = source
            .read_changes(
                &[root],
                &[checkpoint],
                50,
                Default::default(),
                &AtomicBool::new(false),
            )
            .expect("downtime candidates");

        let result = &batch.roots[0];
        assert!(result.fallback_code.is_none());
        assert_eq!(result.observations.len(), 3);
        assert!(result.observations.iter().all(|observation| {
            observation.scope == LibraryChangeScope::Path
                && !observation.relative_path.contains(':')
                && !observation.relative_path.starts_with('/')
        }));
        let rename = result
            .observations
            .iter()
            .find(|observation| observation.relative_path == "new.jpg")
            .expect("paired rename candidate");
        assert_eq!(
            rename.kind,
            LibraryChangeObservationKind::Renamed {
                is_reliably_paired: true,
            }
        );
        assert_eq!(rename.previous_relative_path.as_deref(), Some("old.jpg"));
        assert_eq!(rename.sequence, 33);
    }

    #[test]
    fn controlled_temp_root_yields_candidates_or_an_explicit_fallback() {
        let directory = tempdir().expect("temporary library root");
        let root_path = directory.path().to_string_lossy().into_owned();
        let root = root("root-a", &root_path);
        let source = WindowsUsnCatchUpSource::production();
        let baseline = source
            .read_changes(
                std::slice::from_ref(&root),
                &[],
                10,
                Default::default(),
                &AtomicBool::new(false),
            )
            .expect("safe baseline or fallback");

        let old_path = directory.path().join("old.jpg");
        let new_path = directory.path().join("new.jpg");
        let removed_path = directory.path().join("removed.jpg");
        fs::write(&old_path, b"first").expect("create fixture");
        fs::rename(&old_path, &new_path).expect("rename fixture");
        fs::write(&new_path, b"second").expect("modify fixture");
        fs::write(&removed_path, b"removed").expect("create removal fixture");
        fs::remove_file(&removed_path).expect("remove fixture");

        let batch = source
            .read_changes(
                &[root],
                &baseline.checkpoints,
                20,
                Default::default(),
                &AtomicBool::new(false),
            )
            .expect("bounded catch-up or fallback");

        assert_eq!(batch.roots.len(), 1);
        let result = &batch.roots[0];
        if result.fallback_code.is_some() {
            assert!(result.observations.is_empty());
        } else {
            assert!(!result.observations.is_empty());
            assert!(result.observations.iter().all(|observation| {
                !observation.relative_path.contains(':')
                    && !observation.relative_path.starts_with('/')
                    && observation.origin == LibraryChangeOrigin::StartupCatchUp
            }));
        }
    }

    #[derive(Clone)]
    struct FakeBackend {
        reads: std::sync::Arc<Mutex<u32>>,
        changes: std::sync::Arc<Vec<ResolvedJournalChange>>,
    }

    impl FakeBackend {
        fn new() -> Self {
            Self::with_changes(vec![
                journal_change("//?/Volume{fixture}/library-a/changed.jpg", 30),
                journal_change("//?/Volume{fixture}/library-b/changed.jpg", 31),
            ])
        }

        fn with_changes(changes: Vec<ResolvedJournalChange>) -> Self {
            Self {
                reads: std::sync::Arc::new(Mutex::new(0)),
                changes: std::sync::Arc::new(changes),
            }
        }

        fn read_count(&self) -> u32 {
            *self.reads.lock().expect("reads")
        }
    }

    impl UsnJournalBackend for FakeBackend {
        fn describe_root(&self, root: &IncrementalCatalogRoot) -> Result<ResolvedRoot, ScanError> {
            Ok(ResolvedRoot {
                root: root.clone(),
                volume: VolumeDescriptor {
                    volume_id: "//?/volume{fixture}".to_owned(),
                    open_path: "\\\\?\\Volume{fixture}".to_owned(),
                },
                filesystem: "NTFS".to_owned(),
                root_guid_path: format!(
                    "//?/Volume{{fixture}}/{}",
                    root.root_id.replace("root-", "library-")
                ),
            })
        }

        fn query_journal(&self, _volume: &VolumeDescriptor) -> Result<JournalMetadata, ScanError> {
            Ok(JournalMetadata {
                journal_id: 12,
                first_usn: 10,
                next_usn: 40,
                minimum_major_version: 2,
                maximum_major_version: 3,
            })
        }

        fn read_journal(
            &self,
            _volume: &VolumeDescriptor,
            _metadata: &JournalMetadata,
            _bounds: JournalReadBounds,
            _cancelled: &AtomicBool,
        ) -> Result<Vec<ResolvedJournalChange>, ScanError> {
            *self.reads.lock().expect("reads") += 1;
            Ok(self.changes.as_ref().clone())
        }
    }

    fn checkpoint_for(
        roots: &[ResolvedRoot],
        journal_id: &str,
        catalog_revision: u64,
    ) -> LibraryChangeCatchUpCheckpoint {
        LibraryChangeCatchUpCheckpoint {
            volume_id: "//?/volume{fixture}".to_owned(),
            journal_id: journal_id.to_owned(),
            next_usn: "20".to_owned(),
            root_set_fingerprint: root_set_fingerprint(roots),
            catalog_revision,
            updated_unix_ms: 1,
        }
    }

    fn journal_change(full_path: &str, usn: i64) -> ResolvedJournalChange {
        journal_change_with_kind(full_path, usn, LibraryChangeObservationKind::Modified)
    }

    fn journal_change_with_kind(
        full_path: &str,
        usn: i64,
        kind: LibraryChangeObservationKind,
    ) -> ResolvedJournalChange {
        ResolvedJournalChange {
            full_path: full_path.to_owned(),
            file_reference: FileReference::V2(usn.to_le_bytes()),
            usn,
            observed_unix_ms: 20,
            kind,
            rename_role: None,
            is_directory: false,
        }
    }

    fn journal_rename_change(
        full_path: &str,
        usn: i64,
        reference: u64,
        rename_role: RenameRole,
    ) -> ResolvedJournalChange {
        ResolvedJournalChange {
            full_path: full_path.to_owned(),
            file_reference: FileReference::V2(reference.to_le_bytes()),
            usn,
            observed_unix_ms: 20,
            kind: match rename_role {
                RenameRole::OldName => LibraryChangeObservationKind::Removed,
                RenameRole::NewName => LibraryChangeObservationKind::Created,
            },
            rename_role: Some(rename_role),
            is_directory: false,
        }
    }

    fn root(root_id: &str, root_path: &str) -> IncrementalCatalogRoot {
        IncrementalCatalogRoot {
            root_id: root_id.to_owned(),
            root_path: root_path.to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            active_scan_id: Some("scan".to_owned()),
            has_running_scan: false,
            catalog_revision: 7,
            last_consistency_audit_unix_ms: None,
        }
    }

    fn v2_record(usn: i64, reference: u64, name: &str, reason: u32) -> Vec<u8> {
        variable_record(
            2,
            usn,
            &reference.to_le_bytes(),
            &1_u64.to_le_bytes(),
            name,
            reason,
        )
    }

    fn v3_record(usn: i64, reference: u128, name: &str, reason: u32) -> Vec<u8> {
        variable_record(
            3,
            usn,
            &reference.to_le_bytes(),
            &1_u128.to_le_bytes(),
            name,
            reason,
        )
    }

    fn variable_record(
        major: u16,
        usn: i64,
        reference: &[u8],
        parent: &[u8],
        name: &str,
        reason: u32,
    ) -> Vec<u8> {
        let (
            minimum,
            parent_offset,
            usn_offset,
            timestamp_offset,
            reason_offset,
            attributes_offset,
            name_length_offset,
            name_offset_offset,
        ) = if major == 2 {
            (60, 16, 24, 32, 40, 52, 56, 58)
        } else {
            (76, 24, 40, 48, 56, 68, 72, 74)
        };
        let name_units = name.encode_utf16().collect::<Vec<_>>();
        let unaligned = minimum + name_units.len() * 2;
        let length = unaligned.next_multiple_of(8);
        let mut record = vec![0_u8; length];
        record[..4].copy_from_slice(&u32::try_from(length).unwrap().to_le_bytes());
        record[4..6].copy_from_slice(&major.to_le_bytes());
        record[8..8 + reference.len()].copy_from_slice(reference);
        record[parent_offset..parent_offset + parent.len()].copy_from_slice(parent);
        record[usn_offset..usn_offset + 8].copy_from_slice(&usn.to_le_bytes());
        record[timestamp_offset..timestamp_offset + 8]
            .copy_from_slice(&WINDOWS_TO_UNIX_EPOCH_100NS.to_le_bytes());
        record[reason_offset..reason_offset + 4].copy_from_slice(&reason.to_le_bytes());
        record[attributes_offset..attributes_offset + 4].copy_from_slice(&0_u32.to_le_bytes());
        record[name_length_offset..name_length_offset + 2]
            .copy_from_slice(&u16::try_from(name_units.len() * 2).unwrap().to_le_bytes());
        record[name_offset_offset..name_offset_offset + 2]
            .copy_from_slice(&u16::try_from(minimum).unwrap().to_le_bytes());
        for (index, unit) in name_units.into_iter().enumerate() {
            let offset = minimum + index * 2;
            record[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }
        record
    }
}
