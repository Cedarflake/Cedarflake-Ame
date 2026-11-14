use super::*;

#[test]
fn removal_before_creation_preserves_the_moved_asset_and_later_deletion_still_removes_it() {
    let source = tempdir().unwrap();
    write_png(&source.path().join("old.png"), 2, 2, [10, 20, 30]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let before = fixture.location("old.png").unwrap();
    let destination = fixture.source.path().join("new");
    fs::create_dir(&destination).unwrap();
    fs::rename(
        fixture.source.path().join("old.png"),
        destination.join("moved.png"),
    )
    .unwrap();

    fixture.enqueue(&[intent(&fixture.root_id, "old.png", None, 1)]);
    let report = fixture.process();
    assert_eq!(report.completed_count, 1);
    assert_eq!(report.retried_count, 0);
    assert!(fixture.location("old.png").is_none());
    let moved = fixture.location("new/moved.png").unwrap();
    assert_eq!(moved.asset_id, before.asset_id);
    assert_eq!(moved.file_identity, before.file_identity);

    fixture.enqueue(&[intent(&fixture.root_id, "new/moved.png", None, 2)]);
    assert_eq!(fixture.process().completed_count, 1);
    assert_eq!(
        fixture.location("new/moved.png").unwrap().asset_id,
        before.asset_id
    );
    fs::remove_file(destination.join("moved.png")).unwrap();
    fixture.enqueue(&[intent(&fixture.root_id, "new/moved.png", None, 3)]);
    assert_eq!(fixture.process().completed_count, 1);
    assert!(fixture.location("new/moved.png").is_none());
}

#[test]
fn creation_before_removal_does_not_leave_the_old_location_visible() {
    let source = tempdir().unwrap();
    write_png(&source.path().join("old.png"), 2, 2, [10, 20, 30]);
    let mut fixture = seed_catalog(source, &["old.png"]);
    let before = fixture.location("old.png").unwrap();
    fs::rename(
        fixture.source.path().join("old.png"),
        fixture.source.path().join("new.png"),
    )
    .unwrap();
    fixture.enqueue(&[intent(&fixture.root_id, "new.png", None, 1)]);
    assert_eq!(fixture.process().completed_count, 1);
    assert_eq!(
        fixture.location("new.png").unwrap().asset_id,
        before.asset_id
    );
    fixture.enqueue(&[intent(&fixture.root_id, "old.png", None, 2)]);
    assert_eq!(fixture.process().completed_count, 1);
    assert!(fixture.location("old.png").is_none());
    assert_eq!(
        fixture.location("new.png").unwrap().asset_id,
        before.asset_id
    );
}
