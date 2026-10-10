use std::cell::RefCell;

use super::SqliteCatalog;

type ReadHook = Box<dyn FnOnce()>;

thread_local! {
    static AFTER_AUTHORITY_CHECK: RefCell<Option<ReadHook>> = RefCell::new(None);
}

struct RestoreHook(Option<ReadHook>);

impl Drop for RestoreHook {
    fn drop(&mut self) {
        AFTER_AUTHORITY_CHECK.with(|slot| *slot.borrow_mut() = self.0.take());
    }
}

impl SqliteCatalog {
    pub(crate) fn with_metadata_inventory_source_read_hook<T>(
        hook: impl FnOnce() + 'static,
        read: impl FnOnce() -> T,
    ) -> T {
        let _restore = RestoreHook(
            AFTER_AUTHORITY_CHECK.with(|slot| slot.borrow_mut().replace(Box::new(hook))),
        );
        read()
    }
}

pub(super) fn after_authority_check() {
    let hook = AFTER_AUTHORITY_CHECK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[test]
fn raw_reads_never_reuse_or_end_a_caller_transaction() {
    use super::{
        LibraryChangeLeaseIdentity, MetadataInventoryRunRequest, MetadataInventorySpoolExecution,
    };
    use crate::domain::{LibraryChangeId, LibraryRootGeneration, MetadataInventoryScope};

    let directory = tempfile::tempdir().expect("isolated catalog");
    let catalog =
        SqliteCatalog::open(directory.path().join("catalog.sqlite3")).expect("open catalog");
    let execution = MetadataInventorySpoolExecution {
        request: MetadataInventoryRunRequest {
            run_id: "caller-owned".to_owned(),
            root_id: "fixture-root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            epoch: 1,
            scope: MetadataInventoryScope::Root,
            started_unix_ms: 1,
        },
        lease: LibraryChangeLeaseIdentity {
            change_id: LibraryChangeId::new(1).expect("positive change id"),
            lease_generation: 1,
        },
    };
    catalog
        .connection
        .execute_batch(
            "CREATE TEMP TABLE source_read_sentinel(value INTEGER);
         BEGIN DEFERRED;
         INSERT INTO source_read_sentinel VALUES (1);",
        )
        .expect("start caller transaction");
    let error = catalog
        .metadata_inventory_spool_is_ready(&execution)
        .expect_err("raw read cannot adopt an unrelated snapshot");
    assert_eq!(error.code, "metadata_inventory_source_transaction_active");
    assert!(!catalog.connection.is_autocommit());
    let count = || {
        catalog
            .connection
            .query_row("SELECT COUNT(*) FROM source_read_sentinel", [], |row| {
                row.get::<_, i64>(0)
            })
            .expect("read caller sentinel")
    };
    assert_eq!(count(), 1);
    catalog
        .connection
        .execute_batch("ROLLBACK")
        .expect("caller rolls back");
    assert_eq!(count(), 0);
    assert!(
        catalog
            .metadata_inventory_spool_is_ready(&execution)
            .is_err()
    );
    assert!(
        catalog.connection.is_autocommit(),
        "failed authority lookup retires its own snapshot"
    );
}
