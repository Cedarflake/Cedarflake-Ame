use std::cell::RefCell;

use crate::domain::{LibraryRootGeneration, MetadataInventoryScope, MetadataInventoryStartRequest};
use crate::ports::MetadataInventoryRepository;

use super::super::SqliteCatalog;

mod cleanup_vm;
mod races;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum WriteOperation {
    BeginNext,
    CleanupTerminal,
}

type BeforeWriteHook = Box<dyn FnOnce(WriteOperation)>;

thread_local! {
    static BEFORE_WRITE: RefCell<Option<BeforeWriteHook>> = const { RefCell::new(None) };
}

pub(super) fn before_write(operation: WriteOperation) {
    let hook = BEFORE_WRITE.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook(operation);
    }
}

fn before_write_scope(hook: impl FnOnce(WriteOperation) + 'static) -> BeforeWriteGuard {
    BEFORE_WRITE.with(|slot| {
        assert!(
            slot.borrow().is_none(),
            "one before-write observer per test thread"
        );
        *slot.borrow_mut() = Some(Box::new(hook));
    });
    BeforeWriteGuard
}

struct BeforeWriteGuard;

impl Drop for BeforeWriteGuard {
    fn drop(&mut self) {
        let _ = BEFORE_WRITE.with(|slot| slot.borrow_mut().take());
    }
}

#[test]
fn begin_next_rejects_an_existing_transaction_without_ending_it() {
    let storage = tempfile::tempdir().expect("generated catalog directory");
    let mut catalog =
        SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("generated catalog");
    catalog
        .connection
        .execute_batch("BEGIN DEFERRED")
        .expect("begin caller transaction");
    let epoch = catalog.completed_write_epoch();

    let error = catalog
        .begin_next_metadata_inventory(&MetadataInventoryStartRequest {
            run_id: "inventory-standalone".to_owned(),
            root_id: "generated-root".to_owned(),
            root_generation: LibraryRootGeneration::initial(),
            scope: MetadataInventoryScope::Root,
            started_unix_ms: 2_000,
        })
        .expect_err("lifecycle cannot read a caller-owned transaction");

    assert_eq!(
        error.code,
        "metadata_inventory_lifecycle_transaction_active"
    );
    assert!(!catalog.connection.is_autocommit());
    assert_eq!(catalog.completed_write_epoch(), epoch);
    catalog
        .connection
        .execute_batch("ROLLBACK")
        .expect("caller retires its transaction");
}

#[test]
fn cleanup_rejects_an_existing_transaction_without_ending_it() {
    let storage = tempfile::tempdir().expect("generated catalog directory");
    let mut catalog =
        SqliteCatalog::open(storage.path().join("catalog.sqlite3")).expect("generated catalog");
    catalog
        .connection
        .execute_batch("BEGIN DEFERRED")
        .expect("begin caller transaction");
    let epoch = catalog.completed_write_epoch();

    let error = catalog
        .cleanup_terminal_metadata_inventories(2_000, 1, 1, Default::default())
        .expect_err("cleanup cannot inspect a caller-owned transaction");

    assert_eq!(
        error.code,
        "metadata_inventory_lifecycle_transaction_active"
    );
    assert!(!catalog.connection.is_autocommit());
    assert_eq!(catalog.completed_write_epoch(), epoch);
    catalog
        .connection
        .execute_batch("ROLLBACK")
        .expect("caller retires its transaction");
}
