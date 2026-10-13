use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::domain::{
    LibraryChangeQueuePolicy, PersistentJournalCheckpoint, PersistentJournalContinuityState,
    PersistentJournalEnrollmentReport, ScanError,
};
use crate::ports::{PersistentJournalRepository, PersistentJournalVolumeReader};

use super::catch_up_persistent_journal_volume;

const MAX_CAPTURE_PAGES: usize = 8;
const CAPTURE_QUANTUM: Duration = Duration::from_millis(200);

pub(super) trait CaptureClock {
    fn elapsed(&self) -> Duration;
    fn observed_unix_ms(&self) -> Result<i64, ScanError>;
}

struct SystemCaptureClock(Instant);

impl CaptureClock for SystemCaptureClock {
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }

    fn observed_unix_ms(&self) -> Result<i64, ScanError> {
        unix_ms()
    }
}

pub(crate) struct JournalCaptureWindowReport {
    pages: Vec<PersistentJournalEnrollmentReport>,
}

impl JournalCaptureWindowReport {
    #[cfg(any(debug_assertions, test))]
    pub(crate) fn reports(&self) -> &[PersistentJournalEnrollmentReport] {
        &self.pages
    }

    pub(crate) fn with_registration(
        mut self,
        registration: PersistentJournalEnrollmentReport,
    ) -> Self {
        if registration.failed_root_count > 0 {
            self.pages.insert(0, registration);
        }
        self
    }
}

impl From<PersistentJournalEnrollmentReport> for JournalCaptureWindowReport {
    fn from(page: PersistentJournalEnrollmentReport) -> Self {
        Self { pages: vec![page] }
    }
}

pub(crate) fn catch_up_persistent_journal_window<Repository, Reader>(
    repository: &mut Repository,
    reader: &Reader,
    checkpoints: &[PersistentJournalCheckpoint],
    observed_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    cancelled: &AtomicBool,
) -> Result<JournalCaptureWindowReport, ScanError>
where
    Repository: PersistentJournalRepository,
    Reader: PersistentJournalVolumeReader,
{
    capture_with_clock(
        repository,
        reader,
        checkpoints,
        observed_unix_ms,
        policy,
        cancelled,
        &SystemCaptureClock(Instant::now()),
    )
}

pub(super) fn capture_with_clock<Repository, Reader>(
    repository: &mut Repository,
    reader: &Reader,
    checkpoints: &[PersistentJournalCheckpoint],
    observed_unix_ms: i64,
    policy: LibraryChangeQueuePolicy,
    cancelled: &AtomicBool,
    clock: &impl CaptureClock,
) -> Result<JournalCaptureWindowReport, ScanError>
where
    Repository: PersistentJournalRepository,
    Reader: PersistentJournalVolumeReader,
{
    let mut checkpoints = checkpoints.to_vec();
    let mut observed_unix_ms = observed_unix_ms;
    let mut pages = Vec::new();
    for _ in 0..MAX_CAPTURE_PAGES {
        if !pages.is_empty()
            && (cancelled.load(Ordering::Acquire) || clock.elapsed() >= CAPTURE_QUANTUM)
        {
            break;
        }
        if !pages.is_empty() && repository.journal_capture_should_yield(observed_unix_ms, policy)? {
            break;
        }
        let page = catch_up_persistent_journal_volume(
            repository,
            reader,
            &checkpoints,
            observed_unix_ms,
            policy,
            cancelled,
        )?;
        let may_continue = page.failed_root_count == 0
            && page.observation_count == 0
            && page.advanced_checkpoint_count > 0;
        pages.push(page);
        if !may_continue || cancelled.load(Ordering::Acquire) || clock.elapsed() >= CAPTURE_QUANTUM
        {
            break;
        }
        let mut persisted = Vec::with_capacity(checkpoints.len());
        for checkpoint in &checkpoints {
            if let Some(persisted_checkpoint) = repository.load_persistent_journal_checkpoint(
                &checkpoint.root_id,
                checkpoint.root_generation,
            )? {
                persisted.push(persisted_checkpoint);
            }
        }
        let Some(next) = unfinished_checkpoints(&checkpoints, &persisted) else {
            break;
        };
        checkpoints = next;
        observed_unix_ms = clock.observed_unix_ms()?;
    }
    Ok(JournalCaptureWindowReport { pages })
}

pub(super) fn unfinished_checkpoints(
    previous: &[PersistentJournalCheckpoint],
    persisted: &[PersistentJournalCheckpoint],
) -> Option<Vec<PersistentJournalCheckpoint>> {
    let mut next = Vec::with_capacity(previous.len());
    for previous in previous {
        let checkpoint = persisted.iter().find(|checkpoint| {
            checkpoint.root_id == previous.root_id
                && checkpoint.root_generation == previous.root_generation
        })?;
        if checkpoint.volume != previous.volume
            || checkpoint.journal_id != previous.journal_id
            || checkpoint.root_file_reference != previous.root_file_reference
            || checkpoint.protocol_version != previous.protocol_version
            || checkpoint.contract_version != previous.contract_version
            || checkpoint.continuity != PersistentJournalContinuityState::CatchingUp
            || checkpoint.next_unread_usn <= previous.next_unread_usn
            || checkpoint.next_unread_usn >= checkpoint.captured_exclusive_end
            || (previous.next_unread_usn < previous.captured_exclusive_end
                && checkpoint.captured_exclusive_end != previous.captured_exclusive_end)
        {
            return None;
        }
        next.push(checkpoint.clone());
    }
    Some(next)
}

fn unix_ms() -> Result<i64, ScanError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .ok_or_else(|| {
            ScanError::new(
                "persistent_journal_capture_clock_invalid",
                "The journal capture time cannot be represented",
            )
        })
}
