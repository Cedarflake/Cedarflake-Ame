use crate::application::{
    cancel_catalog_reclamation, cancel_preview_cleanup, clear_previews, clear_retired_previews,
    load_catalog_reclamation, load_storage_status as load_status,
    update_storage_settings as update_settings,
};
use crate::domain::{
    CatalogReclamationSnapshot, PreviewCleanupEvent, ScanError, StorageSettingsUpdate,
    StorageStatus,
};
use crate::frb_generated::StreamSink;

pub fn load_storage_status() -> Result<StorageStatus, ScanError> {
    load_status()
}

pub fn update_storage_settings(update: StorageSettingsUpdate) -> Result<StorageStatus, ScanError> {
    update_settings(update)
}

pub fn start_catalog_database_reclamation(
    operation_id: String,
) -> Result<CatalogReclamationSnapshot, ScanError> {
    crate::application::start_catalog_reclamation(operation_id)
}

pub fn load_catalog_database_reclamation() -> Result<CatalogReclamationSnapshot, ScanError> {
    load_catalog_reclamation()
}

pub fn cancel_catalog_database_reclamation(operation_id: String) -> bool {
    cancel_catalog_reclamation(&operation_id)
}

pub fn clear_preview_cache(
    operation_id: String,
    sink: StreamSink<PreviewCleanupEvent>,
) -> Result<(), ScanError> {
    clear_preview_cache_with(operation_id, |event| sink.add(event).is_ok())
}

pub fn clear_retired_preview_cache(
    preview_root: String,
    operation_id: String,
    sink: StreamSink<PreviewCleanupEvent>,
) -> Result<(), ScanError> {
    clear_retired_preview_cache_with(preview_root, operation_id, |event| sink.add(event).is_ok())
}

fn clear_preview_cache_with(
    operation_id: String,
    mut emit: impl FnMut(PreviewCleanupEvent) -> bool,
) -> Result<(), ScanError> {
    let failure_operation_id = operation_id.clone();
    if let Err(error) = clear_previews(operation_id, &mut emit) {
        let _ = emit(PreviewCleanupEvent::Failed {
            operation_id: failure_operation_id,
            code: error.code,
            message: error.message,
        });
    }
    Ok(())
}

fn clear_retired_preview_cache_with(
    preview_root: String,
    operation_id: String,
    mut emit: impl FnMut(PreviewCleanupEvent) -> bool,
) -> Result<(), ScanError> {
    let failure_operation_id = operation_id.clone();
    if let Err(error) = clear_retired_previews(preview_root, operation_id, &mut emit) {
        let _ = emit(PreviewCleanupEvent::Failed {
            operation_id: failure_operation_id,
            code: error.code,
            message: error.message,
        });
    }
    Ok(())
}

#[flutter_rust_bridge::frb(sync)]
pub fn cancel_preview_cache_cleanup(operation_id: String) -> bool {
    cancel_preview_cleanup(&operation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_errors_are_emitted_as_terminal_failures() {
        let mut events = Vec::new();

        clear_preview_cache_with(" ".to_owned(), |event| {
            events.push(event);
            true
        })
        .expect("cleanup API converts application errors into events");

        assert!(matches!(
            events.as_slice(),
            [PreviewCleanupEvent::Failed {
                operation_id,
                code,
                ..
            }] if operation_id == " " && code == "preview_cleanup_id_empty"
        ));
    }
}
