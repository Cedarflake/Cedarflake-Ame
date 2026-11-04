use std::cell::RefCell;

use crate::domain::{
    ExpectedFileState, LibraryRootAvailability, RootAvailabilityEvidence, ScanIssue,
};

use super::*;

struct RootProbe {
    observed_paths: RefCell<Vec<String>>,
}

impl SourceStateProbe for RootProbe {
    fn inspect_root(&self, root_path: &str) -> RootAvailabilityEvidence {
        self.observed_paths.borrow_mut().push(root_path.to_owned());
        match root_path {
            "unavailable" => RootAvailabilityEvidence {
                availability: LibraryRootAvailability::Offline,
                message: Some("Source unavailable".to_owned()),
            },
            "available" => RootAvailabilityEvidence {
                availability: LibraryRootAvailability::Available,
                message: None,
            },
            _ => panic!("unexpected root probe"),
        }
    }

    fn revalidate_file(&self, _: &ExpectedFileState) -> Result<(), ScanIssue> {
        panic!("root projection must not inspect individual source files")
    }
}

#[test]
fn unavailable_root_does_not_interrupt_projection_or_retain_stale_messages() {
    let probe = RootProbe {
        observed_paths: RefCell::new(Vec::new()),
    };
    let mut roots = [root("unavailable"), root("available")];

    refresh(&probe, &mut roots);

    assert_eq!(roots[0].availability, LibraryRootAvailability::Offline);
    assert_eq!(
        roots[0].availability_message.as_deref(),
        Some("Source unavailable")
    );
    assert_eq!(roots[1].availability, LibraryRootAvailability::Available);
    assert_eq!(roots[1].availability_message, None);
    assert_eq!(*probe.observed_paths.borrow(), ["unavailable", "available"]);
    for root in &roots {
        assert_eq!(root.active_scan_id.as_deref(), Some("published-scan"));
        assert_eq!(root.asset_count, 12);
        assert_eq!(root.issue_count, 1);
    }
}

#[test]
fn native_projection_distinguishes_missing_roots_and_preserves_source_bytes() {
    let source = tempfile::tempdir().expect("owned source fixture");
    let file = source.path().join("保留.txt");
    std::fs::write(&file, b"source bytes").expect("source fixture");
    let missing = source.path().join("missing");
    let mut roots = [
        root(&source.path().to_string_lossy()),
        root(&missing.to_string_lossy()),
    ];

    refresh(&super::super::production::source_state(), &mut roots);

    assert_eq!(roots[0].availability, LibraryRootAvailability::Available);
    assert_eq!(roots[0].availability_message, None);
    assert_eq!(roots[1].availability, LibraryRootAvailability::Missing);
    assert!(roots[1].availability_message.is_some());
    assert!(!missing.exists());
    assert_eq!(
        std::fs::read(file).expect("unchanged source"),
        b"source bytes"
    );
}

fn root(path: &str) -> LibraryRootView {
    LibraryRootView {
        root_id: format!("root-{path}"),
        path: path.to_owned(),
        display_path: path.to_owned(),
        active_scan_id: Some("published-scan".to_owned()),
        created_unix_ms: 7,
        asset_count: 12,
        issue_count: 1,
        availability: LibraryRootAvailability::Unknown,
        availability_message: Some("Previous status".to_owned()),
    }
}
