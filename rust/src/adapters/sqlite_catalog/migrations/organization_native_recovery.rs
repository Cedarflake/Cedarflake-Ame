use rusqlite::Connection;

use crate::domain::ScanError;

use super::schema_shape;

const INDEX: (&str, &str) = (
    "organization_bindings_native_version",
    "CREATE INDEX organization_bindings_native_version ON organization_bindings(
       file_identity_scheme, file_identity_value, source_revision_token, byte_length, record_id)",
);

pub(super) fn validate(connection: &Connection) -> Result<(), ScanError> {
    if !schema_shape::schema_object_sql_matches(connection, "index", INDEX.0, INDEX.1)? {
        return Err(ScanError::new(
            "catalog_organization_native_recovery_invalid",
            "Organization recovery requires the native-version evidence index",
        ));
    }
    Ok(())
}
