use rusqlite::Connection;

use super::super::{
    METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_DDL,
    METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V31_DDL,
    METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL,
    METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL,
    METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL, METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL,
    METADATA_INVENTORY_SPOOL_ENTRY_TABLE_V31_DDL, METADATA_INVENTORY_SPOOL_TABLE_DDL,
};

pub(in crate::adapters::sqlite_catalog::migrations) fn downgrade_to_v31_for_test(
    connection: &Connection,
) {
    let version: i64 = connection
        .query_row("SELECT version FROM schema_info", [], |row| row.get(0))
        .expect("fixture version");
    if version != 32 {
        return;
    }
    let transaction = connection
        .unchecked_transaction()
        .expect("fixture downgrade transaction");
    let names = [
        "library_metadata_inventory_spools",
        "library_metadata_inventory_spool_directories",
        "library_metadata_inventory_spool_entries",
    ];
    for name in names {
        transaction
            .execute_batch(&format!(
                "CREATE TEMP TABLE {name}_fixture AS SELECT * FROM {name}"
            ))
            .expect("retain fixture rows");
    }
    for (name, _) in super::GUARDS {
        transaction
            .execute_batch(&format!("DROP TRIGGER {name}"))
            .expect("remove v32 guard");
    }
    for name in names.iter().rev() {
        transaction
            .execute_batch(&format!("DROP TABLE {name}"))
            .expect("remove v32 storage graph");
    }
    transaction
        .execute_batch("DROP TABLE library_metadata_inventory_spool_contract")
        .expect("remove v32 marker");
    for ddl in [
        METADATA_INVENTORY_SPOOL_CONTRACT_TABLE_V31_DDL,
        METADATA_INVENTORY_SPOOL_TABLE_DDL,
        METADATA_INVENTORY_SPOOL_DIRECTORY_TABLE_DDL,
        METADATA_INVENTORY_SPOOL_ENTRY_TABLE_V31_DDL,
        METADATA_INVENTORY_SPOOL_DIRECTORY_STATE_INDEX_DDL,
        METADATA_INVENTORY_SPOOL_ENTRY_ORDER_INDEX_DDL,
        METADATA_INVENTORY_SPOOL_BINDING_UPDATE_GUARD_DDL,
        METADATA_INVENTORY_SPOOL_DIRECTORY_COMPLETE_GUARD_DDL,
    ] {
        transaction
            .execute_batch(ddl)
            .expect("restore exact v31 DDL");
    }
    // Historical terminal-spool repair fixtures deliberately retain the old ready header.
    transaction.execute_batch("UPDATE library_metadata_inventory_spools_fixture SET state = 'ready' WHERE state = 'retired'")
        .expect("restore historical terminal fixture state");
    for name in names {
        transaction
            .execute_batch(&format!(
                "INSERT INTO {name} SELECT * FROM {name}_fixture; DROP TABLE {name}_fixture"
            ))
            .expect("restore fixture rows");
    }
    transaction
        .execute_batch(
            "INSERT INTO library_metadata_inventory_spool_contract VALUES (1, 3, 1);
        UPDATE schema_info SET version = 31; PRAGMA user_version = 31",
        )
        .expect("restore fixture version");
    transaction.commit().expect("publish v31 fixture");
}
