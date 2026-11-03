use crate::domain::ScanError;
use crate::domain::gallery_metadata_filter::GalleryMetadataFilter;

use super::super::{codec::Cell, error};

pub(super) fn validate(values: &[Cell]) -> Result<(), ScanError> {
    let Some(Cell::Text(payload)) = values.get(9) else {
        return Err(invalid());
    };
    if payload.len() > 4096 {
        return Err(invalid());
    }
    let filter: GalleryMetadataFilter = serde_json::from_str(payload).map_err(|_| invalid())?;
    filter.validate().map_err(|_| invalid())
}

fn invalid() -> ScanError {
    error(
        "records_invalid",
        "The backup contains invalid metadata filters",
    )
}
