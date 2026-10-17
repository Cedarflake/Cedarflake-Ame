use super::*;

#[derive(Clone, Copy)]
pub(super) struct HealthyFactory;

pub(super) struct HealthySource;

pub(super) struct CloseProbeSession {
    pub(super) close_count: Arc<std::sync::atomic::AtomicUsize>,
    pub(super) close_failures: Arc<std::sync::atomic::AtomicUsize>,
}

pub(super) struct BlockingCloseSession {
    pub(super) close_count: Arc<std::sync::atomic::AtomicUsize>,
    pub(super) close_entered: Arc<AtomicBool>,
    pub(super) close_release: Arc<AtomicBool>,
    pub(super) close_finished: Arc<AtomicBool>,
    pub(super) panic_after_release: bool,
}

pub(super) struct BlockingJournalFactory {
    pub(super) connect_entered: Arc<AtomicBool>,
    pub(super) connect_release: Arc<AtomicBool>,
    pub(super) connect_count: Arc<std::sync::atomic::AtomicUsize>,
    pub(super) close_count: Arc<std::sync::atomic::AtomicUsize>,
}

impl crate::ports::LibraryChangeSourceFactory for HealthyFactory {
    type Source = HealthySource;

    fn start(
        &self,
        _request: &crate::ports::LibraryChangeSourceRequest,
    ) -> Result<Self::Source, crate::domain::LibraryChangeSourceError> {
        Ok(HealthySource)
    }
}

impl crate::ports::LibraryChangeSource for HealthySource {
    fn health(&self) -> crate::domain::LibraryChangeSourceHealth {
        crate::domain::LibraryChangeSourceHealth::Healthy
    }

    fn drain(
        &mut self,
        _max_observations: usize,
    ) -> Result<crate::domain::LibraryChangeSourceBatch, crate::domain::LibraryChangeSourceError>
    {
        Ok(crate::domain::LibraryChangeSourceBatch {
            observations: Vec::new(),
            health: crate::domain::LibraryChangeSourceHealth::Healthy,
            dropped_observation_count: 0,
            ignored_callback_count: 0,
            last_issue_code: None,
        })
    }

    fn stop(
        &mut self,
    ) -> Result<crate::domain::LibraryChangeSourceStopReport, crate::domain::LibraryChangeSourceError>
    {
        Ok(crate::domain::LibraryChangeSourceStopReport::default())
    }
}

impl PersistentChangeJournalSession for CloseProbeSession {
    fn register_root(
        &self,
        _request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn query_journal(
        &self,
        _request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn begin_read_range(
        &self,
        _request: ReadJournalRangeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn begin_read_volume(
        &self,
        _request: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        self.close_count.fetch_add(1, Ordering::AcqRel);
        if self
            .close_failures
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                remaining.checked_sub(1)
            })
            .is_ok()
        {
            return Err(PersistentChangeJournalOperationError::TransportUnavailable);
        }
        Ok(())
    }
}

impl PersistentChangeJournalSession for BlockingCloseSession {
    fn register_root(
        &self,
        _request: RegisterRootRequest,
    ) -> Result<RootCapability, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn query_journal(
        &self,
        _request: QueryJournalRequest,
    ) -> Result<BrokerResponse, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn begin_read_range(
        &self,
        _request: ReadJournalRangeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn begin_read_volume(
        &self,
        _request: ReadJournalVolumeRequest,
    ) -> Result<Box<dyn PersistentChangeJournalRead>, PersistentChangeJournalOperationError> {
        Err(PersistentChangeJournalOperationError::InvalidRequest)
    }

    fn close(&self) -> Result<(), PersistentChangeJournalOperationError> {
        self.close_count.fetch_add(1, Ordering::AcqRel);
        self.close_entered.store(true, Ordering::Release);
        while !self.close_release.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        self.close_finished.store(true, Ordering::Release);
        assert!(!self.panic_after_release, "controlled journal close panic");
        Ok(())
    }
}

impl PersistentChangeJournal for BlockingJournalFactory {
    fn connect(&self) -> PersistentChangeJournalConnection {
        self.connect_count.fetch_add(1, Ordering::AcqRel);
        self.connect_entered.store(true, Ordering::Release);
        while !self.connect_release.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
            close_count: Arc::clone(&self.close_count),
            close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }))
    }
}

pub(super) fn test_live_only_connection() -> PersistentChangeJournalConnection {
    PersistentChangeJournalConnection::LiveOnly(PersistentChangeJournalLiveOnlyReason::BrokerAbsent)
}

pub(super) fn runtime_with_close_probe(
    close_count: Arc<std::sync::atomic::AtomicUsize>,
) -> ProductionSynchronization {
    new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
            close_count,
            close_failures: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        })),
    )
}

pub(super) fn runtime_with_blocking_close(
    close_count: Arc<std::sync::atomic::AtomicUsize>,
    close_entered: Arc<AtomicBool>,
    close_release: Arc<AtomicBool>,
    close_finished: Arc<AtomicBool>,
) -> ProductionSynchronization {
    new_production_synchronization_with_connection(
        crate::ports::erase_library_change_source_factory(HealthyFactory),
        PersistentChangeJournalConnection::Connected(Arc::new(BlockingCloseSession {
            close_count,
            close_entered,
            close_release,
            close_finished,
            panic_after_release: false,
        })),
    )
}

pub(super) fn wait_for_test_signal(signal: &AtomicBool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while !signal.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(
        signal.load(Ordering::Acquire),
        "blocking operation did not start"
    );
}

pub(super) struct CountingJournalFactory {
    pub(super) connect_count: Arc<std::sync::atomic::AtomicUsize>,
    pub(super) close_count: Arc<std::sync::atomic::AtomicUsize>,
    pub(super) close_failures: Arc<std::sync::atomic::AtomicUsize>,
}

impl PersistentChangeJournal for CountingJournalFactory {
    fn connect(&self) -> PersistentChangeJournalConnection {
        self.connect_count.fetch_add(1, Ordering::AcqRel);
        PersistentChangeJournalConnection::Connected(Arc::new(CloseProbeSession {
            close_count: Arc::clone(&self.close_count),
            close_failures: Arc::clone(&self.close_failures),
        }))
    }
}
