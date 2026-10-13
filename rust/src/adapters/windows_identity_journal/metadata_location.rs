use std::path::Path;

use crate::domain::{FileIdentityEvidence, JournalFileReference, LibraryRootGeneration, ScanError};
use crate::journal_broker::describe_production_persistent_journal_root;

use super::native::NativeRoot;

pub(crate) fn current_identity_paths(
    root_id: &str,
    root_generation: LibraryRootGeneration,
    root_path: &str,
    expected_root_identity: &FileIdentityEvidence,
    file_identity: &FileIdentityEvidence,
) -> Result<Option<Vec<String>>, ScanError> {
    if file_identity.scheme != "windows-file-id-128-v1" {
        return Ok(None);
    }
    let (volume_text, file_text) = file_identity.value.split_once(':').ok_or_else(invalid)?;
    if volume_text.len() != 16 || file_text.len() != 32 {
        return Err(invalid());
    }
    let volume_serial = u64::from_str_radix(volume_text, 16).map_err(|_| invalid())?;
    let file_id = u128::from_str_radix(file_text, 16).map_err(|_| invalid())?;
    let reference = JournalFileReference::from_bytes(&file_id.to_le_bytes())?;
    let registration = describe_production_persistent_journal_root(
        root_id,
        root_generation.value(),
        Path::new(root_path),
    )
    .map_err(|error| {
        ScanError::new(
            "source_reconciliation_root_unavailable",
            format!("The source root metadata could not be validated: {error}"),
        )
    })?;
    let root = NativeRoot::open(&registration.authorization)?;
    if root.volume.volume_serial != volume_serial {
        return Err(invalid());
    }
    let guard = root.guard()?;
    if guard.metadata_inventory_root_identity()?.as_ref() != Some(expected_root_identity) {
        return Err(invalid());
    }
    let Some(locations) = root.locations(&reference)? else {
        return Ok(None);
    };
    if locations
        .iter()
        .any(|location| location.is_directory || &location.identity != file_identity)
    {
        return Err(invalid());
    }
    Ok(Some(
        locations
            .into_iter()
            .map(|location| location.relative_path)
            .collect(),
    ))
}

fn invalid() -> ScanError {
    ScanError::new(
        "source_reconciliation_identity_invalid",
        "The source file identity could not be resolved inside its current root",
    )
}
