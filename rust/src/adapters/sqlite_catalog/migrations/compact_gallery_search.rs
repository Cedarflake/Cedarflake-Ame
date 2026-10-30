use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const DOCUMENT_COLUMN: &str = "gallery_search_id INTEGER CHECK(gallery_search_id IS NULL OR (
    typeof(gallery_search_id) = 'integer' AND gallery_search_id > 0
))";
const DOCUMENT_INDEX: &str =
    "CREATE UNIQUE INDEX asset_locations_gallery_search ON asset_locations(gallery_search_id)";
const TERMS: &str = "CREATE VIRTUAL TABLE gallery_search_terms USING fts5(
    absolute_path, relative_path, tokenize='trigram', content='', contentless_delete=1,
    detail=none
)";
const INSERT_TRIGGER: &str = "CREATE TRIGGER gallery_search_insert
AFTER INSERT ON asset_locations BEGIN
    UPDATE asset_locations SET gallery_search_id = (
      SELECT COALESCE(MAX(gallery_search_id), 0) + 1 FROM asset_locations
    ) WHERE scan_id = NEW.scan_id AND location_id = NEW.location_id;
    INSERT INTO gallery_search_terms(rowid, absolute_path, relative_path)
      SELECT gallery_search_id, NEW.absolute_path, NEW.relative_path FROM asset_locations
      WHERE scan_id = NEW.scan_id AND location_id = NEW.location_id;
END";
const UPDATE_TRIGGER: &str = "CREATE TRIGGER gallery_search_update
AFTER UPDATE OF absolute_path, relative_path ON asset_locations
WHEN OLD.absolute_path IS NOT NEW.absolute_path OR OLD.relative_path IS NOT NEW.relative_path
BEGIN
    UPDATE gallery_search_terms SET absolute_path = NEW.absolute_path, relative_path = NEW.relative_path
      WHERE rowid = NEW.gallery_search_id;
END";
const DELETE_TRIGGER: &str = "CREATE TRIGGER gallery_search_delete
BEFORE DELETE ON asset_locations BEGIN
    DELETE FROM gallery_search_terms WHERE rowid = OLD.gallery_search_id;
END";
const IDENTITY_TRIGGER: &str = "CREATE TRIGGER gallery_search_identity
BEFORE UPDATE OF gallery_search_id ON asset_locations
WHEN OLD.gallery_search_id IS NOT NULL AND OLD.gallery_search_id IS NOT NEW.gallery_search_id
BEGIN SELECT RAISE(ABORT, 'catalog search identity is immutable'); END";

const OBJECTS: [(&str, &str, &str); 6] = [
    ("index", "asset_locations_gallery_search", DOCUMENT_INDEX),
    ("table", "gallery_search_terms", TERMS),
    ("trigger", "gallery_search_insert", INSERT_TRIGGER),
    ("trigger", "gallery_search_update", UPDATE_TRIGGER),
    ("trigger", "gallery_search_delete", DELETE_TRIGGER),
    ("trigger", "gallery_search_identity", IDENTITY_TRIGGER),
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 62)?;
    transaction
        .execute_batch(&format!(
            "DROP TRIGGER gallery_search_insert;
             DROP TRIGGER gallery_search_update;
             DROP TRIGGER gallery_search_delete;
             DROP TABLE gallery_search_terms;
             ALTER TABLE asset_locations ADD COLUMN {DOCUMENT_COLUMN};
             UPDATE asset_locations SET gallery_search_id = (
               SELECT id FROM gallery_search_documents AS documents
               WHERE documents.scan_id = asset_locations.scan_id
                 AND documents.location_id = asset_locations.location_id
             );"
        ))
        .map_err(database_error)?;
    validate_rows(transaction)?;
    transaction
        .execute_batch("DROP TABLE gallery_search_documents;")
        .map_err(database_error)?;
    for (_, _, sql) in OBJECTS {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    backfill_terms(transaction)?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 63; PRAGMA user_version = 63;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 63)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, sql) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, sql)? {
            return Err(invalid());
        }
    }
    let sql: String = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'asset_locations'",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if !schema_shape::normalize_schema_sql(&sql)
        .contains(&schema_shape::normalize_schema_sql(DOCUMENT_COLUMN))
    {
        return Err(invalid());
    }
    Ok(())
}

fn backfill_terms(connection: &Connection) -> Result<(), ScanError> {
    connection
        .execute_batch(
            "INSERT INTO gallery_search_terms(rowid, absolute_path, relative_path)
             SELECT gallery_search_id, absolute_path, relative_path FROM asset_locations;",
        )
        .map_err(database_error)
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let missing: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM asset_locations WHERE gallery_search_id IS NULL)",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if missing {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_search_index_invalid",
        "The catalog text search index requires its canonical identity, table and triggers",
    )
}

#[cfg(test)]
pub(super) fn downgrade_to_v62_for_test(connection: &Connection) {
    super::organization_review_filters::downgrade_to_v63_for_test(connection);
    if schema_shape::schema_version(connection).unwrap() != 63 {
        return;
    }
    for (kind, name, _) in OBJECTS {
        connection
            .execute_batch(&format!("DROP {kind} {name}"))
            .unwrap();
    }
    for (_, _, sql) in super::gallery_search::OBJECTS {
        connection.execute_batch(sql).unwrap();
    }
    connection
        .execute_batch(
            "INSERT INTO gallery_search_documents(id, scan_id, location_id)
           SELECT gallery_search_id, scan_id, location_id FROM asset_locations;
         INSERT INTO gallery_search_terms(rowid, absolute_path, relative_path)
           SELECT gallery_search_id, absolute_path, relative_path FROM asset_locations;
         ALTER TABLE asset_locations DROP COLUMN gallery_search_id;
         UPDATE schema_info SET version = 62; PRAGMA user_version = 62;",
        )
        .unwrap();
}

#[cfg(test)]
mod tests;
