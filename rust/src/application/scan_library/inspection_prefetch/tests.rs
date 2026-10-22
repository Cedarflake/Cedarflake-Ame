use std::sync::atomic::AtomicUsize;
use std::time::{Duration, Instant};

use super::*;
use crate::application::scan_library::CONTROL_PAUSE;

fn visit(path: String) -> InspectedVisit {
    InspectedVisit {
        visit: FileVisit {
            relative_path: path,
            outcome: FileVisitOutcome::Ignored,
        },
        inspection: ScanInspection::Deferred,
    }
}

#[test]
fn resumed_and_published_scans_keep_prior_driven_inspection() {
    let mut request = ScanRequest {
        scan_id: "admission".to_owned(),
        root_path: String::new(),
        max_items: None,
        max_entries: None,
        preview_edge: 128,
    };
    assert!(
        InspectionMode::for_scan(&request, FullScanReason::ExplicitUserRequest, false, 0)
            == InspectionMode::Prefetch
    );
    assert!(
        InspectionMode::for_scan(
            &request,
            FullScanReason::ResumeForegroundCheckpoint,
            false,
            0
        ) == InspectionMode::Serial
    );
    assert!(
        InspectionMode::for_scan(&request, FullScanReason::ExplicitUserRequest, true, 0)
            == InspectionMode::Serial
    );
    assert!(
        InspectionMode::for_scan(&request, FullScanReason::ExplicitUserRequest, false, 1)
            == InspectionMode::Serial
    );
    request.max_items = Some(1);
    assert!(
        InspectionMode::for_scan(&request, FullScanReason::ExplicitUserRequest, false, 0)
            == InspectionMode::Serial
    );
    request.max_items = None;
    request.max_entries = Some(1);
    assert!(
        InspectionMode::for_scan(&request, FullScanReason::ExplicitUserRequest, false, 0)
            == InspectionMode::Serial
    );
}

#[test]
fn producer_preserves_path_order_and_joins_at_end() {
    let control = AtomicU8::new(CONTROL_RUNNING);
    std::thread::scope(|scope| {
        let paths: Vec<_> = (0..33).map(|index| index.to_string()).collect();
        let mut prefetch = InspectionPrefetch::spawn(scope, paths.clone(), &control, visit)
            .expect("start inspection");
        let received: Vec<_> = prefetch
            .by_ref()
            .map(|entry| entry.expect("inspection result").visit.relative_path)
            .collect();
        assert_eq!(received, paths);
        assert!(prefetch.worker.is_none());
        assert!(prefetch.next().is_none());
    });
}

#[test]
fn dropping_a_full_queue_stops_and_joins_the_blocked_producer() {
    let control = AtomicU8::new(CONTROL_RUNNING);
    let calls = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let prefetch = InspectionPrefetch::spawn(
            scope,
            (0..100).map(|index| index.to_string()).collect(),
            &control,
            |path| {
                calls.fetch_add(1, Ordering::Release);
                visit(path)
            },
        )
        .expect("start inspection");
        let deadline = Instant::now() + Duration::from_secs(5);
        while calls.load(Ordering::Acquire) < PREFETCH_CAPACITY + 1 && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert_eq!(calls.load(Ordering::Acquire), PREFETCH_CAPACITY + 1);
        drop(prefetch);
        assert_eq!(calls.load(Ordering::Acquire), PREFETCH_CAPACITY + 1);
    });
}

#[test]
fn pause_prevents_admitting_the_next_source() {
    let control = AtomicU8::new(CONTROL_RUNNING);
    let calls = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        let mut prefetch = InspectionPrefetch::spawn(
            scope,
            vec!["first".to_owned(), "second".to_owned()],
            &control,
            |path| {
                calls.fetch_add(1, Ordering::Release);
                control.store(CONTROL_PAUSE, Ordering::Release);
                visit(path)
            },
        )
        .expect("start inspection");
        assert_eq!(
            prefetch.next().unwrap().unwrap().visit.relative_path,
            "first"
        );
        assert!(prefetch.next().is_none());
        assert_eq!(calls.load(Ordering::Acquire), 1);
    });
}

#[test]
fn worker_panic_is_an_error_instead_of_a_complete_inventory() {
    let control = AtomicU8::new(CONTROL_RUNNING);
    std::thread::scope(|scope| {
        let mut prefetch = InspectionPrefetch::spawn(
            scope,
            vec!["first".to_owned(), "panic".to_owned()],
            &control,
            |path| {
                assert_ne!(path, "panic", "controlled worker failure");
                visit(path)
            },
        )
        .expect("start inspection");
        assert!(prefetch.next().unwrap().is_ok());
        let error = match prefetch.next().expect("worker failure") {
            Err(error) => error,
            Ok(_) => panic!("worker failure must not produce an entry"),
        };
        assert_eq!(error.code, "scan_inspection_worker_failed");
        assert!(prefetch.next().is_none());
    });
}
