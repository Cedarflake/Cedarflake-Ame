use std::cell::{Cell, RefCell};

use crate::domain::{
    AssetLocationView, FileIdentityEvidence, IncrementalCatalogRoot, LibraryRootGeneration,
    PreviewStatus, RootAvailabilityEvidence, SourceRevisionEvidence,
};

use super::*;

struct Probe {
    root: LibraryRootAvailability,
    issue: Option<&'static str>,
    roots: Cell<usize>,
    files: RefCell<Vec<ExpectedFileState>>,
}

impl SourceStateProbe for Probe {
    fn inspect_root(&self, root_path: &str) -> RootAvailabilityEvidence {
        assert_eq!(root_path, "source-root");
        assert!(self.files.borrow().is_empty());
        self.roots.set(self.roots.get() + 1);
        RootAvailabilityEvidence {
            availability: self.root,
            message: None,
        }
    }

    fn revalidate_file(&self, expected: &ExpectedFileState) -> Result<(), ScanIssue> {
        assert_eq!(self.roots.get(), 1);
        self.files.borrow_mut().push(expected.clone());
        match self.issue {
            None => Ok(()),
            Some(code) => Err(ScanIssue {
                path: Some(expected.absolute_path.clone()),
                code: code.to_owned(),
                message: "fixture".to_owned(),
            }),
        }
    }
}

#[test]
fn unavailable_roots_do_not_probe_candidate_files() {
    for (root, expected) in [
        (
            LibraryRootAvailability::Offline,
            DuplicateLocationAvailability::Offline,
        ),
        (
            LibraryRootAvailability::Missing,
            DuplicateLocationAvailability::Unavailable,
        ),
        (
            LibraryRootAvailability::Inaccessible,
            DuplicateLocationAvailability::Unavailable,
        ),
    ] {
        let probe = Probe {
            root,
            issue: None,
            roots: Cell::new(0),
            files: RefCell::new(Vec::new()),
        };
        assert_eq!(inspect(&probe, &candidate()), expected);
        assert_eq!(probe.roots.get(), 1);
        assert!(probe.files.borrow().is_empty());
    }
}

#[test]
fn admitted_roots_revalidate_the_complete_candidate_state() {
    for root in [
        LibraryRootAvailability::Available,
        LibraryRootAvailability::Unknown,
    ] {
        for (issue, expected) in [
            (None, DuplicateLocationAvailability::Local),
            (
                Some("source_became_unavailable"),
                DuplicateLocationAvailability::CloudOnly,
            ),
            (
                Some("source_revision_changed"),
                DuplicateLocationAvailability::Unavailable,
            ),
        ] {
            let candidate = candidate();
            let probe = Probe {
                root,
                issue,
                roots: Cell::new(0),
                files: RefCell::new(Vec::new()),
            };
            assert_eq!(inspect(&probe, &candidate), expected);
            let files = probe.files.borrow();
            assert_eq!(files.len(), 1);
            let observed = &files[0];
            let location = &candidate.location;
            assert_eq!(observed.absolute_path, location.absolute_path);
            assert_eq!(observed.file_size, location.file_size);
            assert_eq!(observed.modified_unix_ms, location.modified_unix_ms);
            assert_eq!(observed.file_identity, location.file_identity);
            assert_eq!(observed.source_revision, location.source_revision);
        }
    }
}

#[test]
fn placeholder_revalidation_remains_distinct_from_missing_or_changed_files() {
    for (code, expected) in [
        (
            "source_became_unavailable",
            DuplicateLocationAvailability::CloudOnly,
        ),
        (
            "cloud_placeholder_skipped",
            DuplicateLocationAvailability::CloudOnly,
        ),
        (
            "source_revalidation_failed",
            DuplicateLocationAvailability::Unavailable,
        ),
        (
            "source_revision_changed",
            DuplicateLocationAvailability::Unavailable,
        ),
    ] {
        let issue = ScanIssue {
            path: None,
            code: code.to_owned(),
            message: "fixture".to_owned(),
        };
        assert_eq!(file_availability(Err(issue)), expected);
    }
    assert_eq!(
        file_availability(Ok(())),
        DuplicateLocationAvailability::Local
    );
}

fn candidate() -> FingerprintCandidate {
    let generation = 7;
    FingerprintCandidate {
        location: AssetLocationView {
            asset_id: format!("asset-{generation}"),
            location_id: format!("location-{generation:04}"),
            root_id: "root".to_owned(),
            scan_id: "scan".to_owned(),
            absolute_path: "source-root\\image.png".to_owned(),
            display_path: "source-root\\image.png".to_owned(),
            relative_path: "image.png".to_owned(),
            preview_path: String::new(),
            file_size: 20,
            created_unix_ms: None,
            modified_unix_ms: 1,
            file_identity: Some(FileIdentityEvidence {
                scheme: "fixture-id".to_owned(),
                value: "stable-id".to_owned(),
            }),
            source_revision: Some(SourceRevisionEvidence {
                scheme: "fixture-revision".to_owned(),
                value: "revision-7".to_owned(),
            }),
            source_generation: generation,
            width: 20,
            height: 30,
            preview_status: PreviewStatus::Pending,
            preview_issue_code: None,
            preview_issue_message: None,
            metadata_engine_id: "fixture".to_owned(),
            metadata_engine_version: "1".to_owned(),
            camera: Default::default(),
            capture_time: None,
        },
        root: IncrementalCatalogRoot {
            root_id: "root".to_owned(),
            root_path: "source-root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            active_scan_id: Some("scan".to_owned()),
            has_running_scan: false,
            catalog_revision: 0,
            last_consistency_audit_unix_ms: None,
            publication_root_identity: None,
        },
    }
}
