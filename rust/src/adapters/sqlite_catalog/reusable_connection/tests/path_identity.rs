use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

use super::{
    Connection, LibraryChangeLane, SqliteCatalogSession, assert_rejected,
    full_schema_validation_count, reset_full_schema_validation_count,
};

struct FixtureJunction {
    path: PathBuf,
}

impl FixtureJunction {
    fn new(parent: &Path, target: &Path) -> Self {
        let junction = Self {
            path: parent.join("current-catalog"),
        };
        junction.create(target);
        junction
    }

    fn create(&self, target: &Path) {
        let output = Command::new("cmd.exe")
            .args(["/C", "mklink", "/J"])
            .arg(&self.path)
            .arg(target)
            .output()
            .expect("create disposable catalog junction");
        assert!(
            output.status.success(),
            "junction creation failed: {output:?}"
        );
    }

    fn retarget(&self, target: &Path) {
        fs::remove_dir(&self.path).expect("unlink only the original junction");
        self.create(target);
    }
}

impl Drop for FixtureJunction {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.path);
    }
}

fn seed_catalog(path: &Path, revision: i64) -> SqliteCatalogSession {
    drop(SqliteCatalogSession::validate(path.to_path_buf()).expect("initialize catalog"));
    let writer = Connection::open(path).expect("open disposable catalog writer");
    assert_eq!(
        writer
            .execute("UPDATE catalog_state SET revision = ?1", [revision])
            .expect("seed distinct catalog revision"),
        1,
    );
    drop(writer);
    reset_full_schema_validation_count(path);
    let session =
        SqliteCatalogSession::validate(path.to_path_buf()).expect("FULL-validate seeded catalog");
    assert_eq!(full_schema_validation_count(path), 1);
    session
}

fn revision(path: &Path) -> i64 {
    assert!(path.is_file());
    Connection::open(path)
        .expect("open existing disposable catalog")
        .query_row("SELECT revision FROM catalog_state", [], |row| row.get(0))
        .expect("read committed catalog revision")
}

fn schema_statements(path: &Path) -> Vec<String> {
    let connection = Connection::open(path).expect("open schema comparison");
    let mut statement = connection
        .prepare("SELECT sql FROM sqlite_schema WHERE sql IS NOT NULL ORDER BY type, name")
        .expect("prepare schema comparison");
    statement
        .query_map([], |row| row.get(0))
        .expect("query schema comparison")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("read schema comparison")
}

#[test]
fn reusable_connection_rejects_junction_retarget_while_original_sqlite_handle_is_alive() {
    let directory = tempfile::tempdir().expect("disposable catalog fixture");
    let target_a = directory.path().join("catalog-a");
    let target_b = directory.path().join("catalog-b");
    fs::create_dir(&target_a).expect("create catalog A directory");
    fs::create_dir(&target_b).expect("create catalog B directory");
    let path_a = target_a.join("catalog.sqlite3");
    let path_b = target_b.join("catalog.sqlite3");
    let session_a = seed_catalog(&path_a, 11);
    let session_b = seed_catalog(&path_b, 29);
    assert_eq!(schema_statements(&path_a), schema_statements(&path_b));
    assert_eq!(
        (
            session_a.application_id,
            session_a.user_version,
            session_a.schema_cookie,
        ),
        (
            session_b.application_id,
            session_b.user_version,
            session_b.schema_cookie,
        ),
    );
    assert_ne!(session_a.database_identity, session_b.database_identity);

    let junction = FixtureJunction::new(directory.path(), &target_a);
    let alias = junction.path.join("catalog.sqlite3");
    let session = SqliteCatalogSession::validate(alias.clone()).expect("FULL-validate through A");
    let catalog = session
        .open_in_lane(LibraryChangeLane::Live)
        .expect("retain original SQLite connection through junction");
    session
        .revalidate_connection(&catalog)
        .expect("original junction target remains current");
    assert_eq!(session.database_identity, session_a.database_identity);

    junction.retarget(&target_b);
    assert_eq!(
        fs::canonicalize(&alias).unwrap(),
        fs::canonicalize(&path_b).unwrap()
    );
    assert_eq!(
        catalog
            .connection
            .query_row("SELECT revision FROM catalog_state", [], |row| row
                .get::<_, i64>(0))
            .expect("original SQLite handle still refers to A"),
        11,
    );
    assert_rejected(&session, &catalog, "catalog_validated_session_stale");
    assert_eq!(
        session
            .validate_connection_proof(&catalog.connection, &session.database_identity)
            .expect_err("the after-proof observation must also detect junction retarget")
            .code,
        "catalog_validated_session_stale",
    );
    assert_eq!((revision(&path_a), revision(&path_b)), (11, 29));
    drop(catalog);
    drop(session);

    let fresh = SqliteCatalogSession::validate(alias).expect("FULL-validate retargeted catalog B");
    assert_eq!(fresh.database_identity, session_b.database_identity);
    let fresh_catalog = fresh
        .open_in_lane(LibraryChangeLane::Live)
        .expect("open catalog B through retargeted junction");
    fresh
        .revalidate_connection(&fresh_catalog)
        .expect("fresh session owns the current target");
    assert_eq!(
        fresh_catalog
            .connection
            .query_row("SELECT revision FROM catalog_state", [], |row| row
                .get::<_, i64>(0))
            .expect("fresh session reads B"),
        29,
    );
    drop(fresh_catalog);
    drop(fresh);
    assert_eq!((revision(&path_a), revision(&path_b)), (11, 29));
    fs::remove_dir(&junction.path).expect("unlink only the final junction");
    drop(junction);
    drop(session_a);
    drop(session_b);
    directory
        .close()
        .expect("remove disposable catalog fixture");
}

#[test]
fn retained_proof_checks_retarget_even_when_the_old_database_schema_is_unreadable() {
    let directory = tempfile::tempdir().unwrap();
    let target_a = directory.path().join("catalog-a");
    let target_b = directory.path().join("catalog-b");
    fs::create_dir(&target_a).unwrap();
    fs::create_dir(&target_b).unwrap();
    let path_a = target_a.join("catalog.sqlite3");
    let path_b = target_b.join("catalog.sqlite3");
    let _session_a = seed_catalog(&path_a, 11);
    let _session_b = seed_catalog(&path_b, 29);
    let junction = FixtureJunction::new(directory.path(), &target_a);
    let session = SqliteCatalogSession::validate(junction.path.join("catalog.sqlite3")).unwrap();
    let catalog = session.open_in_lane(LibraryChangeLane::Live).unwrap();
    catalog
        .connection
        .execute_batch("DROP TABLE schema_info")
        .unwrap();
    junction.retarget(&target_b);
    assert_rejected(&session, &catalog, "catalog_validated_session_stale");
    assert_eq!(revision(&path_b), 29);
}

#[test]
fn retained_proof_rejects_namespace_changes_during_successful_or_failed_sql_observation() {
    for deny_sql in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let target_a = directory.path().join("catalog-a");
        let target_b = directory.path().join("catalog-b");
        fs::create_dir(&target_a).unwrap();
        fs::create_dir(&target_b).unwrap();
        let path_a = target_a.join("catalog.sqlite3");
        let path_b = target_b.join("catalog.sqlite3");
        let _session_a = seed_catalog(&path_a, 11);
        let _session_b = seed_catalog(&path_b, 29);
        let junction = Arc::new(FixtureJunction::new(directory.path(), &target_a));
        let session =
            SqliteCatalogSession::validate(junction.path.join("catalog.sqlite3")).unwrap();
        let catalog = session.open_in_lane(LibraryChangeLane::Live).unwrap();
        let changed = Arc::new(AtomicBool::new(false));
        let during_sql = Arc::clone(&changed);
        let during_junction = Arc::clone(&junction);
        catalog
            .connection
            .authorizer(Some(move |context: AuthContext<'_>| {
                if matches!(
                    context.action,
                    AuthAction::Read {
                        table_name: "schema_info",
                        ..
                    }
                ) && !during_sql.swap(true, Ordering::AcqRel)
                {
                    during_junction.retarget(&target_b);
                    if deny_sql {
                        return Authorization::Deny;
                    }
                }
                Authorization::Allow
            }))
            .unwrap();
        assert_rejected(&session, &catalog, "catalog_validated_session_stale");
        assert!(
            changed.load(Ordering::Acquire),
            "retarget occurred inside SQL proof"
        );
        assert_eq!(
            catalog
                .connection
                .query_row("SELECT revision FROM catalog_state", [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            11
        );
        assert_eq!(revision(&path_b), 29);
    }
}

#[test]
fn retained_proof_rejects_a_different_wal_namespace_with_the_same_database_file_id() {
    let directory = tempfile::tempdir().unwrap();
    let target_a = directory.path().join("catalog-a");
    let target_b = directory.path().join("catalog-b");
    fs::create_dir(&target_a).unwrap();
    fs::create_dir(&target_b).unwrap();
    let path_a = target_a.join("catalog.sqlite3");
    let path_b = target_b.join("catalog.sqlite3");
    let _session_a = seed_catalog(&path_a, 11);
    fs::hard_link(&path_a, &path_b).unwrap();
    let original = crate::adapters::read_catalog_identity(&path_a).unwrap();
    let alias = crate::adapters::read_catalog_identity(&path_b).unwrap();
    assert_eq!(original.file_identity, alias.file_identity);
    assert_ne!(original.canonical_path, alias.canonical_path);
    let junction = FixtureJunction::new(directory.path(), &target_a);
    let session = SqliteCatalogSession::validate(junction.path.join("catalog.sqlite3")).unwrap();
    let catalog = session.open_in_lane(LibraryChangeLane::Live).unwrap();
    junction.retarget(&target_b);
    assert_rejected(&session, &catalog, "catalog_validated_session_stale");
    assert!(!target_b.join("catalog.sqlite3-wal").exists());
    assert!(!target_b.join("catalog.sqlite3-shm").exists());
}
