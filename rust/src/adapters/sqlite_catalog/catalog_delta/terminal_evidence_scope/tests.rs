use super::*;

#[test]
fn file_lease_does_not_authorize_its_previous_path_or_children() {
    let scope = CompletedLeasePaths::new(
        "path",
        "rename_candidate",
        "new.png".into(),
        Some("old.png".into()),
    )
    .unwrap();
    assert!(scope.accepts_terminal_path("new.png"));
    for path in ["old.png", "new.png/child", "other.png"] {
        assert!(!scope.accepts_terminal_path(path));
    }
}

#[test]
fn renamed_subtrees_authorize_only_complete_path_components() {
    let scope = CompletedLeasePaths::new(
        "subtree",
        "rename_candidate",
        "new".into(),
        Some("old".into()),
    )
    .unwrap();
    for path in ["new", "new/a.png", "new/nested/b.png", "old/a.png"] {
        assert!(scope.accepts_terminal_path(path));
    }
    for path in [
        "newer/a.png",
        "older/a.png",
        "other.png",
        "new/../other.png",
    ] {
        assert!(!scope.accepts_terminal_path(path));
    }
}

#[test]
fn root_scope_still_requires_normalized_relative_paths() {
    let scope = CompletedLeasePaths::new("root", "freshness_unknown", String::new(), None).unwrap();
    assert!(scope.accepts_terminal_path("album/image.png"));
    for path in [
        "",
        "/image.png",
        "../image.png",
        "a/./b.png",
        "a//b.png",
        "C:/image.png",
        "a\\b.png",
        "image.png:stream",
    ] {
        assert!(!scope.accepts_terminal_path(path));
    }
    assert!(CompletedLeasePaths::new("unknown", "reconcile", String::new(), None).is_err());
    assert!(
        CompletedLeasePaths::new("subtree", "reconcile", "new".into(), Some("old".into())).is_err()
    );
    assert!(CompletedLeasePaths::new("subtree", "rename_candidate", "new".into(), None).is_err());
}

#[test]
fn terminal_scope_retirement_is_literal_case_sensitive_and_bounded() {
    let mut connection = rusqlite::Connection::open_in_memory().unwrap();
    connection
        .execute_batch(
            "CREATE TABLE library_terminal_media_evidence (
           root_id TEXT NOT NULL, relative_path TEXT NOT NULL,
           PRIMARY KEY (root_id, relative_path)
         );",
        )
        .unwrap();
    for (root, path) in [
        ("root", "album_%"),
        ("root", "album_%/bad.png"),
        ("root", "album_%2/neighbor.png"),
        ("root", "album_x/bad.png"),
        ("root", "Album_%/bad.png"),
        ("peer", "album_%/bad.png"),
    ] {
        connection
            .execute(
                "INSERT INTO library_terminal_media_evidence VALUES (?1, ?2)",
                params![root, path],
            )
            .unwrap();
    }
    let transaction = connection.transaction().unwrap();
    let scope = CompletedLeasePaths::new("subtree", "reconcile", "album_%".into(), None).unwrap();
    assert_eq!(
        scope.retained_terminal_paths(&transaction, "root").unwrap(),
        ["album_%", "album_%/bad.png"]
    );
    for index in 0..MAX_DELTA_MUTATIONS {
        transaction
            .execute(
                "INSERT INTO library_terminal_media_evidence VALUES ('root', ?1)",
                [format!("album_%/extra-{index}.png")],
            )
            .unwrap();
    }
    assert_eq!(
        scope
            .retained_terminal_paths(&transaction, "root")
            .unwrap_err()
            .code,
        "metadata_inventory_required"
    );
}
