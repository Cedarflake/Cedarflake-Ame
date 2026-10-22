use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

use super::*;

#[test]
fn scan_prior_lookup_reuses_queries_for_distinct_unseen_files() {
    let directory = tempdir().expect("isolated storage");
    let catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    let preparations = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&preparations);
    catalog
        .connection
        .authorizer(Some(move |context: AuthContext<'_>| {
            if matches!(context.action, AuthAction::Select) {
                observed.fetch_add(1, Ordering::Relaxed);
            }
            Authorization::Allow
        }))
        .expect("observe SQL compilation");

    for index in 0..256 {
        let prior = catalog
            .load_scan_location_by_file_identity(
                &format!("scan-{index}"),
                &FileIdentityEvidence {
                    scheme: format!("scheme-{}", index % 2),
                    value: index.to_string(),
                },
            )
            .expect("lookup unseen file");
        assert!(prior.is_none());
    }
    assert_eq!(preparations.load(Ordering::Relaxed), 3);
}

#[test]
fn cached_prior_lookup_observes_publication_and_later_commits() {
    let directory = tempdir().expect("isolated storage");
    let path = directory.path().join("catalog.sqlite3");
    let mut catalog = SqliteCatalog::open(path.clone()).expect("open catalog");
    let identity = FileIdentityEvidence {
        scheme: "fixture".to_owned(),
        value: "physical-file".to_owned(),
    };
    assert!(
        catalog
            .load_scan_location_by_file_identity("next-scan", &identity)
            .expect("lookup before publication")
            .is_none()
    );

    let mut item = location("asset", "location", "root", "before.jpg");
    item.file_identity = Some(identity.clone());
    seed_catalog(&mut catalog, "root", "C:/source", &[item]);
    let prior = catalog
        .load_scan_location_by_file_identity("next-scan", &identity)
        .expect("lookup after publication")
        .expect("published location");
    assert_eq!(prior.relative_path, "before.jpg");
    assert_eq!(prior.width, 64);

    let writer = rusqlite::Connection::open(&path).expect("independent writer");
    writer
        .execute(
            "UPDATE asset_locations SET width = 128 WHERE location_id = 'location'",
            [],
        )
        .expect("commit new metadata");
    let prior = catalog
        .load_scan_location_by_file_identity("next-scan", &identity)
        .expect("lookup after commit")
        .expect("current location");
    assert_eq!(prior.width, 128);

    let missing_identity = FileIdentityEvidence {
        scheme: identity.scheme,
        value: "other-file".to_owned(),
    };
    assert!(
        catalog
            .load_scan_location_by_file_identity("next-scan", &missing_identity)
            .expect("rebind physical identity")
            .is_none()
    );
    assert!(
        catalog
            .load_incremental_location_by_relative_path("another-root", "before.jpg")
            .expect("different root")
            .is_none()
    );
    assert_eq!(
        catalog
            .load_incremental_location_by_relative_path("root", "before.jpg")
            .expect("rebind root")
            .expect("root location")
            .width,
        128
    );
}
