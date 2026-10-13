use std::fs;

use rusqlite::{Connection, OpenFlags};

use crate::application::StoragePaths;
use crate::domain::{ScanEvent, ScanRequest};
use crate::media_fixtures::{MediaFixtureFormat, encode_rgb_quadrants, truncated_header};

use super::super::{cancel_scan, pause_scan, run_scan_with_storage};

#[test]
fn pause_and_cancel_retain_issues_added_during_final_validation() {
    for cancel in [false, true] {
        let source = tempfile::tempdir().expect("disposable source");
        let derived = tempfile::tempdir().expect("isolated catalog");
        let source_path = source.path().join("writing.bmp");
        fs::write(&source_path, truncated_header(MediaFixtureFormat::Bmp))
            .expect("incomplete header");
        let completed_bytes =
            encode_rgb_quadrants(MediaFixtureFormat::Bmp, 32, 24).expect("complete source pixels");
        let request = ScanRequest {
            scan_id: format!(
                "final-validation-control-{}-{cancel}",
                super::super::stable_id("fixture", &source.path().to_string_lossy())
            ),
            root_path: source.path().to_string_lossy().into_owned(),
            max_items: None,
            max_entries: None,
            preview_edge: 128,
        };
        let storage = StoragePaths {
            catalog_path: derived.path().join("catalog.sqlite3"),
            preview_root: derived.path().join("previews"),
            preview_budget_bytes: 1024 * 1024,
            settings_path: derived.path().join("settings.sqlite3"),
        };
        let mut events = Vec::new();
        let mut control_accepted = false;
        run_scan_with_storage(
            request.clone(),
            |event| {
                if let ScanEvent::Issue { issue, .. } = &event {
                    match issue.code.as_str() {
                        "image_decode_invalid" => fs::write(&source_path, &completed_bytes)
                            .expect("external writer completes fixture"),
                        "source_changed_during_scan" => {
                            control_accepted = if cancel {
                                cancel_scan(&request.scan_id)
                            } else {
                                pause_scan(&request.scan_id)
                            };
                        }
                        code => panic!("unexpected scan issue {code}"),
                    }
                }
                events.push(event);
                true
            },
            storage.clone(),
        )
        .expect("controlled scan");
        assert!(
            control_accepted,
            "control must target the live final-validation execution"
        );
        let issues = events
            .iter()
            .filter(|event| matches!(event, ScanEvent::Issue { .. }))
            .count();
        assert_eq!(issues, 2);
        match events.last().expect("terminal event") {
            ScanEvent::Paused { issue_count, .. } if !cancel => assert_eq!(*issue_count, 2),
            ScanEvent::Cancelled { issue_count, .. } if cancel => assert_eq!(*issue_count, 2),
            event => panic!("expected controlled terminal state, got {event:?}"),
        }
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, ScanEvent::Completed { .. }))
        );
        let connection =
            Connection::open_with_flags(&storage.catalog_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .expect("read-only terminal evidence");
        let (status, stored_count, issue_rows, active_roots): (String, i64, i64, i64) = connection.query_row(
            "SELECT status, issue_count, (SELECT COUNT(*) FROM scan_issues WHERE scan_id = ?1),
                (SELECT COUNT(*) FROM library_roots WHERE active_scan_id = ?1) FROM scan_runs WHERE id = ?1",
            [&request.scan_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).expect("persisted scan and issue counts");
        assert_eq!(status, if cancel { "cancelled" } else { "paused" });
        assert_eq!((stored_count, issue_rows, active_roots), (2, 2, 0));
        assert_eq!(
            fs::read(&source_path).expect("source is not modified by control"),
            completed_bytes
        );
    }
}
