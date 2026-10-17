use rusqlite::Connection;

use crate::adapters::SqliteCatalog;

use super::super::retired_live_gap_repair::test_catalog::{
    assert_retired_live_gap_claim_repaired, retire_live_gap_root,
    seed_current_explicit_live_gap_claim,
};
use super::super::terminal_inventory_repair::test_catalog::{
    assert_current_terminal_inventory_repaired, current_terminal_inventory_authority_catalog,
};

#[test]
fn current_schema_repairs_inventory_and_live_gap_residue_atomically() {
    let catalog = current_terminal_inventory_authority_catalog();
    let connection = Connection::open(catalog.path()).expect("open mixed repair fixture");
    seed_current_explicit_live_gap_claim(&connection, "mixed-removed-root", 4_031);
    retire_live_gap_root(&connection, "mixed-removed-root", 4_031);
    drop(connection);

    drop(SqliteCatalog::open(catalog.path().to_path_buf()).expect("repair mixed terminal state"));
    let evidence = Connection::open(catalog.path()).expect("inspect mixed repair evidence");
    assert_current_terminal_inventory_repaired(&evidence);
    assert_retired_live_gap_claim_repaired(&evidence, 4_031);
}
