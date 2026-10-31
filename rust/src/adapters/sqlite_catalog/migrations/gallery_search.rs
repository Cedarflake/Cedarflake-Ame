use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;

use super::{current_schema, database_error, schema_shape};

const DOCUMENTS: &str = "CREATE TABLE gallery_search_documents (
    id INTEGER PRIMARY KEY,
    scan_id TEXT NOT NULL,
    location_id TEXT NOT NULL,
    UNIQUE(scan_id, location_id),
    FOREIGN KEY(scan_id, location_id) REFERENCES asset_locations(scan_id, location_id)
      ON UPDATE CASCADE ON DELETE CASCADE
)";

pub(super) const TERMS: &str = "CREATE VIRTUAL TABLE gallery_search_terms USING fts5(
    absolute_path, relative_path, tokenize='trigram', content='', contentless_delete=1
)";

const INSERT_TRIGGER: &str = "CREATE TRIGGER gallery_search_insert
AFTER INSERT ON asset_locations BEGIN
    INSERT INTO gallery_search_documents(scan_id, location_id)
      VALUES (NEW.scan_id, NEW.location_id);
    INSERT INTO gallery_search_terms(rowid, absolute_path, relative_path)
      SELECT id, NEW.absolute_path, NEW.relative_path FROM gallery_search_documents
      WHERE scan_id = NEW.scan_id AND location_id = NEW.location_id;
END";

const UPDATE_TRIGGER: &str = "CREATE TRIGGER gallery_search_update
AFTER UPDATE OF scan_id, location_id, absolute_path, relative_path ON asset_locations
WHEN OLD.scan_id IS NOT NEW.scan_id OR OLD.location_id IS NOT NEW.location_id
  OR OLD.absolute_path IS NOT NEW.absolute_path OR OLD.relative_path IS NOT NEW.relative_path
BEGIN
    UPDATE gallery_search_documents SET scan_id = NEW.scan_id, location_id = NEW.location_id
      WHERE scan_id = OLD.scan_id AND location_id = OLD.location_id;
    UPDATE gallery_search_terms SET absolute_path = NEW.absolute_path, relative_path = NEW.relative_path
      WHERE rowid = (SELECT id FROM gallery_search_documents
        WHERE scan_id = NEW.scan_id AND location_id = NEW.location_id);
END";

const DELETE_TRIGGER: &str = "CREATE TRIGGER gallery_search_delete
BEFORE DELETE ON asset_locations BEGIN
    DELETE FROM gallery_search_terms WHERE rowid = (SELECT id FROM gallery_search_documents
      WHERE scan_id = OLD.scan_id AND location_id = OLD.location_id);
    DELETE FROM gallery_search_documents WHERE scan_id = OLD.scan_id AND location_id = OLD.location_id;
END";

pub(super) const OBJECTS: [(&str, &str, &str); 5] = [
    ("table", "gallery_search_documents", DOCUMENTS),
    ("table", "gallery_search_terms", TERMS),
    ("trigger", "gallery_search_insert", INSERT_TRIGGER),
    ("trigger", "gallery_search_update", UPDATE_TRIGGER),
    ("trigger", "gallery_search_delete", DELETE_TRIGGER),
];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 60)?;
    for (_, _, sql) in OBJECTS {
        transaction.execute_batch(sql).map_err(database_error)?;
    }
    // The stable integer document key survives VACUUM; asset_locations has a composite text key.
    transaction
        .execute_batch(
            "INSERT INTO gallery_search_documents(scan_id, location_id)
               SELECT scan_id, location_id FROM asset_locations;
             INSERT INTO gallery_search_terms(rowid, absolute_path, relative_path)
               SELECT documents.id, locations.absolute_path, locations.relative_path
               FROM gallery_search_documents AS documents
               JOIN asset_locations AS locations USING(scan_id, location_id);
             UPDATE schema_info SET version = 61; PRAGMA user_version = 61;",
        )
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 61)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    for (kind, name, sql) in OBJECTS {
        if !schema_shape::schema_object_sql_matches(connection, kind, name, sql)? {
            return Err(ScanError::new(
                "catalog_search_index_invalid",
                "The catalog text search index requires its canonical tables and triggers",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn downgrade_to_v60_for_test(connection: &Connection) {
    super::camera_metadata::downgrade_to_v61_for_test(connection);
    if schema_shape::schema_version(connection).unwrap() == 61 {
        connection
            .execute_batch(
                "DROP TRIGGER gallery_search_insert;
                 DROP TRIGGER gallery_search_update;
                 DROP TRIGGER gallery_search_delete;
                 DROP TABLE gallery_search_terms;
                 DROP TABLE gallery_search_documents;
                 UPDATE schema_info SET version = 60; PRAGMA user_version = 60;",
            )
            .unwrap();
    }
}

#[cfg(test)]
mod tests;
