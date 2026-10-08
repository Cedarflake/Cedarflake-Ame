use super::*;

struct AuditFixture {
    base: Fixture,
    peer_root: String,
    peer_path: PathBuf,
}

impl AuditFixture {
    fn new() -> Self {
        let base = Fixture::new();
        let peer_path = base._source.path().join("peer");
        fs::create_dir(&peer_path).unwrap();
        fs::copy(
            base.original.join("historical.png"),
            peer_path.join("historical.png"),
        )
        .unwrap();
        run_scan_with_storage(request("peer", &peer_path), |_| true, base.storage.clone()).unwrap();
        let peer_root = base
            .snapshot()
            .roots
            .into_iter()
            .find(|root| root.active_scan_id.as_deref() == Some("peer"))
            .unwrap()
            .root_id;
        fs::create_dir(&base.replacement).unwrap();
        Self {
            base,
            peer_root,
            peer_path,
        }
    }

    fn run(&self, limit: usize) -> Result<AuditEvidence, String> {
        audit(
            &self.base.storage.catalog_path,
            [&self.base.root.root_id, &self.peer_root],
            &self.base.replacement,
            limit,
        )
    }
}

#[test]
fn identity_audit_distinguishes_moves_copies_and_unobserved_old_files_without_mutation() {
    let fixture = AuditFixture::new();
    let moved = fixture.base.replacement.join("moved.png");
    let copied = fixture.base.replacement.join("copied.png");
    fs::rename(fixture.base.original.join("historical.png"), &moved).unwrap();
    fs::copy(fixture.peer_path.join("historical.png"), &copied).unwrap();
    let catalog_before = fs::read(&fixture.base.storage.catalog_path).unwrap();
    let sources: Vec<_> = [&moved, &copied, &fixture.peer_path.join("historical.png")]
        .into_iter()
        .map(|path| (path.clone(), fs::read(path).unwrap()))
        .collect();
    let report = fixture.run(MAX_ENTRIES).unwrap();
    assert!(report.complete);
    assert_eq!(report.files, 2);
    assert_eq!(report.roots[0].unique_identity_observed, 1);
    assert_eq!(report.roots[0].identity_not_observed, 0);
    assert_eq!(report.roots[1].unique_identity_observed, 0);
    assert_eq!(report.roots[1].identity_not_observed, 1);
    assert_eq!(report.unrecognized_file_identity, 1);
    assert_eq!(
        fs::read(&fixture.base.storage.catalog_path).unwrap(),
        catalog_before
    );
    for (path, bytes) in sources {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn identity_audit_retains_ambiguity_and_refuses_partial_traversal_as_completion() {
    let fixture = AuditFixture::new();
    let alias = fixture.peer_path.join("alias.png");
    fs::hard_link(fixture.base.original.join("historical.png"), &alias).unwrap();
    run_scan_with_storage(
        request("peer-with-alias", &fixture.peer_path),
        |_| true,
        fixture.base.storage.clone(),
    )
    .unwrap();
    fs::rename(
        fixture.base.original.join("historical.png"),
        fixture.base.replacement.join("moved.png"),
    )
    .unwrap();
    fs::copy(
        fixture.peer_path.join("historical.png"),
        fixture.base.replacement.join("copy.png"),
    )
    .unwrap();
    let report = fixture.run(MAX_ENTRIES).unwrap();
    assert!(report.complete);
    assert_eq!(report.roots[0].unique_identity_observed, 0);
    assert_eq!(report.roots[0].ambiguous_identity_observed, 1);
    assert_eq!(report.roots[1].ambiguous_identity_observed, 1);
    let partial = fixture.run(1).unwrap();
    assert!(!partial.complete);
    assert_eq!(partial.entries, 1);
    assert!(fixture.run(0).is_err());
    assert!(fixture.run(MAX_ENTRIES + 1).is_err());
}

#[test]
fn identity_audit_preserves_revision_scheme_and_recognizes_unchanged_metadata() {
    let fixture = AuditFixture::new();
    let result = audit(
        &fixture.base.storage.catalog_path,
        [&fixture.base.root.root_id, &fixture.peer_root],
        &fixture.base.original,
        MAX_ENTRIES,
    )
    .unwrap();
    assert!(result.complete);
    assert_eq!(result.roots[0].metadata_unchanged, 1);
    assert_eq!(result.roots[0].metadata_changed_or_unknown, 0);
    fs::hard_link(
        fixture.base.original.join("historical.png"),
        fixture.base.replacement.join("first.png"),
    )
    .unwrap();
    fs::hard_link(
        fixture.base.original.join("historical.png"),
        fixture.base.replacement.join("second.png"),
    )
    .unwrap();
    let aliases = fixture.run(MAX_ENTRIES).unwrap();
    assert!(aliases.complete);
    assert_eq!(aliases.roots[0].unique_identity_observed, 0);
    assert_eq!(aliases.roots[0].ambiguous_identity_observed, 1);
}

#[test]
fn identity_audit_does_not_follow_a_junction_or_report_its_subtree_as_complete() {
    let fixture = AuditFixture::new();
    let link = fixture.base.replacement.join("linked-directory");
    let source_path = fixture.peer_path.join("historical.png");
    let source_bytes = fs::read(&source_path).unwrap();
    let result = std::process::Command::new("cmd.exe")
        .args(["/D", "/C", "mklink", "/J"])
        .arg(&link)
        .arg(&fixture.peer_path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "generated junction preparation failed"
    );
    let report = fixture.run(MAX_ENTRIES);
    fs::remove_dir(&link).unwrap();
    let report = report.unwrap();
    assert!(!report.complete);
    assert_eq!(report.skipped_entries, 1);
    assert_eq!(report.files, 0);
    assert_eq!(fs::read(source_path).unwrap(), source_bytes);
}
