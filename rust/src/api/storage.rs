use crate::application::{
    load_storage_status as load_status, update_storage_settings as update_settings,
};
use crate::domain::{ScanError, StorageSettingsUpdate, StorageStatus};

#[flutter_rust_bridge::frb(sync)]
pub fn load_storage_status() -> Result<StorageStatus, ScanError> {
    load_status()
}

#[flutter_rust_bridge::frb(sync)]
pub fn update_storage_settings(update: StorageSettingsUpdate) -> Result<StorageStatus, ScanError> {
    update_settings(update)
}
