use super::*;

#[derive(Clone, Copy, Debug)]
enum ConnectionLifetime {
    PerPoll,
    PerEpoch,
}

struct ConnectionControl {
    lifetime: ConnectionLifetime,
    polls: usize,
    opens: usize,
    closes: usize,
    poll_retirement: Duration,
    last_owner: Option<PollCatalogOwner>,
}

impl ConnectionControl {
    fn new(lifetime: ConnectionLifetime) -> Self {
        Self {
            lifetime,
            polls: 0,
            opens: 0,
            closes: 0,
            poll_retirement: Duration::ZERO,
            last_owner: None,
        }
    }

    fn poll(
        &mut self,
        runtime: &mut ProductionSynchronization,
        storage: &crate::application::storage::StoragePaths,
    ) -> Result<LibrarySynchronizationSnapshot, ScanError> {
        let before = runtime.poll_catalog.connection_counts();
        let result = poll_runtime_with_storage(runtime, storage);
        // Only the control changes lifetime. The production path, catalog proof, workload,
        // and outer event-to-visible stopwatch are identical in both arms.
        let retirement = match self.lifetime {
            ConnectionLifetime::PerPoll => {
                let owner = std::mem::take(&mut runtime.poll_catalog);
                let started = Instant::now();
                let retirement = owner.retire();
                self.poll_retirement += started.elapsed();
                self.last_owner = Some(owner);
                retirement
            }
            ConnectionLifetime::PerEpoch => {
                self.last_owner = Some(runtime.poll_catalog.clone());
                Ok(())
            }
        };
        let after = self.last_owner.as_ref().unwrap().connection_counts();
        self.polls += 1;
        self.opens += after.0 - before.0;
        self.closes += after.1 - before.1;
        result.and_then(|snapshot| retirement.map(|()| snapshot))
    }

    fn assert_lifetime_and_report(&self, p95: Duration) {
        assert!(self.polls >= 25);
        match self.lifetime {
            ConnectionLifetime::PerPoll => {
                assert_eq!(self.opens, self.polls);
                assert_eq!(self.closes, self.polls);
            }
            ConnectionLifetime::PerEpoch => {
                assert_eq!(self.opens, 1);
                assert_eq!(self.closes, 0);
                assert_eq!(
                    self.last_owner.as_ref().unwrap().connection_counts(),
                    (1, 1)
                );
            }
        }
        eprintln!(
            "controlled poll lifetime={:?} polls={} poll_catalog_opens={} \
             poll_catalog_closes={} poll_retirement_ms={} visible_p95_ms={}",
            self.lifetime,
            self.polls,
            self.opens,
            self.closes,
            self.poll_retirement.as_millis(),
            p95.as_millis(),
        );
    }
}

#[test]
fn same_mixed_load_controls_poll_connection_lifetime_without_relaxing_the_product_gate() {
    let mut per_poll = ConnectionControl::new(ConnectionLifetime::PerPoll);
    let per_poll_p95 = run_priority_workload(|runtime, storage| per_poll.poll(runtime, storage));
    per_poll.assert_lifetime_and_report(per_poll_p95);

    let mut per_epoch = ConnectionControl::new(ConnectionLifetime::PerEpoch);
    let per_epoch_p95 = run_priority_workload(|runtime, storage| per_epoch.poll(runtime, storage));
    per_epoch.assert_lifetime_and_report(per_epoch_p95);
    // The removed policy is a measured control, not an alternative product acceptance path.
    assert!(
        per_epoch_p95 <= Duration::from_secs(1),
        "P0 P95 was {per_epoch_p95:?}"
    );
}
