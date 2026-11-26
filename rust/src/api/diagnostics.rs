use crate::domain::storage_versions::{CATALOG_SCHEMA_VERSION, SETTINGS_SCHEMA_VERSION};

pub struct DiagnosticStorageVersions {
    pub catalog: i64,
    pub settings: i64,
}

#[flutter_rust_bridge::frb(sync)]
pub fn diagnostic_storage_versions() -> DiagnosticStorageVersions {
    DiagnosticStorageVersions {
        catalog: CATALOG_SCHEMA_VERSION,
        settings: SETTINGS_SCHEMA_VERSION,
    }
}
