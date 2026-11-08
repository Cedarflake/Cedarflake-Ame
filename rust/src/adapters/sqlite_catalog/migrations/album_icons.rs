use rusqlite::{Connection, Transaction, TransactionBehavior};

use crate::domain::ScanError;
use crate::domain::organization::OrganizationAlbumIcon;

use super::{current_schema, database_error, schema_shape};

const ICONS: &str = "CREATE TABLE organization_album_icons (
    album_id TEXT PRIMARY KEY NOT NULL REFERENCES organization_albums(album_id) ON DELETE CASCADE,
    icon TEXT NOT NULL CHECK(icon IN ('album', 'travel', 'nature', 'pets', 'people', 'food',
        'art', 'work', 'study', 'games', 'music', 'memories'))
)";

pub(super) fn migrate(connection: &mut Connection) -> Result<(), ScanError> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    migrate_transaction(&transaction)?;
    transaction.commit().map_err(database_error)
}

pub(super) fn migrate_transaction(transaction: &Transaction<'_>) -> Result<(), ScanError> {
    current_schema::validate_schema_version(transaction, 77)?;
    transaction.execute_batch(ICONS).map_err(database_error)?;
    transaction
        .execute_batch("UPDATE schema_info SET version = 78; PRAGMA user_version = 78;")
        .map_err(database_error)?;
    current_schema::validate_schema_version(transaction, 78)
}

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_shape::schema_object_sql_matches(
        connection,
        "table",
        "organization_album_icons",
        ICONS,
    )? {
        return Err(invalid());
    }
    Ok(())
}

pub(super) fn validate_rows(connection: &Connection) -> Result<(), ScanError> {
    let orphaned: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_check('organization_album_icons'))",
            [],
            |row| row.get(0),
        )
        .map_err(database_error)?;
    if orphaned {
        return Err(invalid());
    }
    let mut statement = connection
        .prepare("SELECT DISTINCT icon FROM organization_album_icons")
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(database_error)?;
    for row in rows {
        if OrganizationAlbumIcon::from_stored(&row.map_err(database_error)?).is_none() {
            return Err(invalid());
        }
    }
    Ok(())
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_album_icon_invalid",
        "Album icons require valid album ownership",
    )
}

#[cfg(test)]
pub(super) fn downgrade_for_test(connection: &Connection) {
    if schema_shape::schema_version(connection).unwrap() == 78 {
        connection.execute_batch("DROP TABLE organization_album_icons; UPDATE schema_info SET version = 77; PRAGMA user_version = 77;").unwrap();
    }
}

#[cfg(test)]
mod tests;
