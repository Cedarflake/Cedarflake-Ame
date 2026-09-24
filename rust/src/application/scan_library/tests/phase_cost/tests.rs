use super::*;

fn started(scan_id: &str) -> ScanEvent {
    ScanEvent::Started {
        scan_id: scan_id.to_owned(),
        root_path: String::new(),
        item_limit: None,
        entry_limit: None,
    }
}

fn finalizing(validated_items: u64) -> ScanEvent {
    ScanEvent::Finalizing {
        scan_id: "cost".to_owned(),
        validated_items,
        total_items: 10_000,
        visited_entries: 10_000,
        accepted_items: 10_000,
        issue_count: 0,
    }
}

fn completed() -> ScanEvent {
    ScanEvent::Completed {
        scan_id: "cost".to_owned(),
        root_id: String::new(),
        asset_count: 10_000,
        issue_count: 0,
        catalog_path: String::new(),
        was_limited: false,
    }
}

fn progress(scan_id: &str) -> ScanEvent {
    ScanEvent::Progress {
        scan_id: scan_id.to_owned(),
        visited_entries: 128,
        accepted_items: 128,
        issue_count: 0,
    }
}

#[test]
fn repeated_complete_counters_preserve_the_first_publication_boundary() {
    let mut cost = ScanPhaseCost::new("cost", 10_000);
    for (event, second) in [
        (started("cost"), 2),
        (finalizing(0), 10),
        (finalizing(10_000), 15),
        (finalizing(10_000), 19),
        (completed(), 23),
    ] {
        cost.observe(&event, Duration::from_secs(second)).unwrap();
    }
    assert_eq!(
        cost.finish(Duration::from_secs(24)).unwrap(),
        ScanPhaseReport {
            admission: Duration::from_secs(2),
            discovery: Duration::from_secs(8),
            validation: Duration::from_secs(5),
            publication: Duration::from_secs(8),
            retirement: Duration::from_secs(1),
            complete_counter_reports: 2,
        }
    );
}

#[test]
fn complete_counters_cannot_stand_in_for_published_completion() {
    let mut cost = ScanPhaseCost::new("cost", 10_000);
    cost.observe(&started("cost"), Duration::ZERO).unwrap();
    cost.observe(&finalizing(0), Duration::ZERO).unwrap();
    cost.observe(&finalizing(10_000), Duration::from_secs(1))
        .unwrap();
    assert_eq!(
        cost.finish(Duration::from_secs(2)),
        Err("missing published completion")
    );
}

#[test]
fn foreign_and_out_of_order_events_are_rejected() {
    let mut cost = ScanPhaseCost::new("cost", 10_000);
    assert!(cost.observe(&started("other"), Duration::ZERO).is_err());
    assert!(cost.observe(&finalizing(0), Duration::ZERO).is_err());
    cost.observe(&started("cost"), Duration::from_secs(1))
        .unwrap();
    assert!(cost.observe(&completed(), Duration::from_secs(2)).is_err());
    assert!(cost.observe(&finalizing(0), Duration::ZERO).is_err());
}

#[test]
fn regressing_or_incomplete_counters_are_rejected() {
    let mut cost = ScanPhaseCost::new("cost", 10_000);
    cost.observe(&started("cost"), Duration::ZERO).unwrap();
    cost.observe(&finalizing(0), Duration::ZERO).unwrap();
    cost.observe(&finalizing(128), Duration::from_secs(1))
        .unwrap();
    assert!(
        cost.observe(&finalizing(127), Duration::from_secs(2))
            .is_err()
    );
    assert!(
        cost.observe(&finalizing(10_001), Duration::from_secs(3))
            .is_err()
    );
    assert!(cost.finish(Duration::from_secs(4)).is_err());
}

#[test]
fn discovery_observations_require_current_identity_and_discovery_phase() {
    let mut cost = ScanPhaseCost::new("cost", 10_000);
    assert!(cost.observe(&progress("cost"), Duration::ZERO).is_err());
    cost.observe(&started("cost"), Duration::ZERO).unwrap();
    assert!(cost.observe(&progress("other"), Duration::ZERO).is_err());
    cost.observe(&progress("cost"), Duration::from_secs(1))
        .unwrap();
    cost.observe(&finalizing(0), Duration::from_secs(2))
        .unwrap();
    assert!(
        cost.observe(&progress("cost"), Duration::from_secs(3))
            .is_err()
    );
}

#[test]
fn successful_workload_rejects_issues_and_unsuccessful_terminal_states() {
    let events = [
        ScanEvent::Cancelled {
            scan_id: "cost".to_owned(),
            accepted_items: 128,
            issue_count: 0,
        },
        ScanEvent::Paused {
            scan_id: "cost".to_owned(),
            visited_entries: 128,
            accepted_items: 128,
            issue_count: 0,
        },
        ScanEvent::Stale {
            scan_id: "cost".to_owned(),
            accepted_items: 128,
            issue_count: 0,
        },
        ScanEvent::Failed {
            scan_id: "cost".to_owned(),
            code: "failed".to_owned(),
            message: String::new(),
        },
        ScanEvent::Issue {
            scan_id: "cost".to_owned(),
            issue: crate::domain::ScanIssue {
                path: None,
                code: "unexpected".to_owned(),
                message: String::new(),
            },
        },
    ];
    for event in events {
        let mut cost = ScanPhaseCost::new("cost", 10_000);
        cost.observe(&started("cost"), Duration::ZERO).unwrap();
        assert!(
            cost.observe(&event, Duration::from_secs(1)).is_err(),
            "{event:?}"
        );
    }
}

#[test]
fn completion_rejects_every_later_observation_and_an_earlier_return() {
    let mut cost = ScanPhaseCost::new("cost", 10_000);
    cost.observe(&started("cost"), Duration::ZERO).unwrap();
    cost.observe(&finalizing(0), Duration::ZERO).unwrap();
    cost.observe(&finalizing(10_000), Duration::from_secs(1))
        .unwrap();
    cost.observe(&completed(), Duration::from_secs(2)).unwrap();
    for event in [
        started("cost"),
        progress("cost"),
        finalizing(10_000),
        completed(),
    ] {
        assert!(cost.observe(&event, Duration::from_secs(3)).is_err());
    }
    assert!(cost.finish(Duration::from_secs(1)).is_err());
}

#[test]
fn validation_requires_the_zero_counter_boundary() {
    let mut cost = ScanPhaseCost::new("cost", 10_000);
    cost.observe(&started("cost"), Duration::ZERO).unwrap();
    assert_eq!(
        cost.observe(&finalizing(10_000), Duration::from_secs(1)),
        Err("missing initial validation boundary")
    );
}

#[test]
fn completion_requires_a_full_issue_free_unlimited_inventory() {
    for (asset_count, issue_count, was_limited) in
        [(9999, 0, false), (10_000, 1, false), (10_000, 0, true)]
    {
        let mut cost = ScanPhaseCost::new("cost", 10_000);
        cost.observe(&started("cost"), Duration::ZERO).unwrap();
        cost.observe(&finalizing(0), Duration::ZERO).unwrap();
        cost.observe(&finalizing(10_000), Duration::from_secs(1))
            .unwrap();
        let mut event = completed();
        if let ScanEvent::Completed {
            asset_count: count,
            issue_count: issues,
            was_limited: limited,
            ..
        } = &mut event
        {
            *count = asset_count;
            *issues = issue_count;
            *limited = was_limited;
        }
        assert!(cost.observe(&event, Duration::from_secs(2)).is_err());
    }
}
