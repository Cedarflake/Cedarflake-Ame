use super::*;

#[test]
fn ordinary_scan_recovers_a_unique_retained_directory_identity() {
    let fixture = Fixture::new();
    let before = fixture.snapshot();
    fs::rename(&fixture.original, &fixture.replacement).expect("move generated root");

    run_scan_with_storage(
        request("discovered-directory", &fixture.replacement),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("scan a discovered source without a manual root selection");

    let after = fixture.snapshot();
    assert_eq!(
        after.roots.len(),
        1,
        "a moved root must not be registered twice"
    );
    assert_eq!(after.roots[0].root_id, fixture.root.root_id);
    assert_eq!(after.assets.len(), 1);
    assert_eq!(after.assets[0].asset_id, before.assets[0].asset_id);
    assert_eq!(after.assets[0].location_id, before.assets[0].location_id);
    let recovered = fixture.current_root();
    assert!(recovered.root_generation > fixture.root.root_generation);
    assert!(Path::new(&recovered.root_path) == fixture.replacement.canonicalize().unwrap());
}

#[test]
fn combined_directory_keeps_both_sources_and_matches_only_proven_file_identity() {
    let fixture = Fixture::new();
    let peer = fixture._source.path().join("second-source");
    fs::create_dir(&peer).expect("peer directory");
    RgbImage::from_pixel(8, 6, Rgb([220, 50, 70]))
        .save(peer.join("historical.png"))
        .expect("same relative name in the second root");
    run_scan_with_storage(request("peer", &peer), |_| true, fixture.storage.clone())
        .expect("publish the second source");
    let before = fixture.snapshot();
    assert_eq!(before.roots.len(), 2);
    let old_local = before
        .assets
        .iter()
        .find(|asset| asset.root_id == fixture.root.root_id)
        .unwrap();
    let old_peer = before
        .assets
        .iter()
        .find(|asset| asset.root_id != fixture.root.root_id)
        .unwrap();

    let moved = fixture.replacement.join("from-first");
    let copied = fixture.replacement.join("from-second");
    fs::create_dir_all(&moved).expect("first destination");
    fs::create_dir(&copied).expect("second destination");
    fs::rename(
        fixture.original.join("historical.png"),
        moved.join("historical.png"),
    )
    .expect("move the first generated file without changing its identity");
    fs::copy(peer.join("historical.png"), copied.join("historical.png"))
        .expect("copy the second file with a distinct identity");
    run_scan_with_storage(
        request("combined", &fixture.replacement),
        |_| true,
        fixture.storage.clone(),
    )
    .expect("publish the newly assembled source");

    let after = fixture.snapshot();
    assert_eq!(after.roots.len(), 3);
    for old_root in &before.roots {
        let retained = after
            .roots
            .iter()
            .find(|root| root.root_id == old_root.root_id)
            .unwrap();
        assert_eq!(retained.path, old_root.path);
        assert_eq!(retained.active_scan_id, old_root.active_scan_id);
    }
    let observed: Vec<_> = after
        .assets
        .iter()
        .filter(|asset| asset.scan_id == "combined")
        .collect();
    assert_eq!(observed.len(), 2);
    let new_local = observed
        .iter()
        .find(|asset| asset.relative_path == "from-first/historical.png")
        .unwrap();
    let new_peer = observed
        .iter()
        .find(|asset| asset.relative_path == "from-second/historical.png")
        .unwrap();
    assert_eq!(new_local.asset_id, old_local.asset_id);
    assert_eq!(new_local.file_identity, old_local.file_identity);
    assert_ne!(new_peer.asset_id, old_peer.asset_id);
    assert_ne!(new_peer.file_identity, old_peer.file_identity);
    assert_eq!(
        fs::read(peer.join("historical.png")).unwrap(),
        fs::read(copied.join("historical.png")).unwrap()
    );
    assert_eq!(
        after.assets.len(),
        4,
        "old snapshots remain until reconciliation"
    );
}

#[test]
fn ambiguous_retained_directory_identity_cannot_choose_a_root_by_order() {
    let fixture = Fixture::new();
    let peer = fixture._source.path().join("ambiguous-peer");
    fs::create_dir(&peer).expect("peer directory");
    run_scan_with_storage(request("peer", &peer), |_| true, fixture.storage.clone())
        .expect("peer root");
    let identity = fixture.root.publication_root_identity.as_ref().unwrap();
    let connection = rusqlite::Connection::open(&fixture.storage.catalog_path).unwrap();
    connection
        .execute(
            "UPDATE library_root_publication_namespaces
             SET identity_scheme = ?1, identity_value = ?2",
            rusqlite::params![identity.scheme, identity.value],
        )
        .expect("arrange ambiguous retained root evidence");
    drop(connection);
    let before = fixture.snapshot();
    fs::rename(&fixture.original, &fixture.replacement).expect("move the generated root");

    let error = run_scan_with_storage(
        request("ambiguous", &fixture.replacement),
        |_| true,
        fixture.storage.clone(),
    )
    .expect_err("ambiguous root evidence must not create or select a root");

    assert_eq!(error.code, "catalog_root_identity_ambiguous");
    let after = fixture.snapshot();
    assert_eq!(after.roots.len(), before.roots.len());
    assert_eq!(after.assets.len(), before.assets.len());
    assert_eq!(fixture.current_root().root_path, fixture.root.root_path);
}
