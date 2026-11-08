use crate::domain::ScanError;
use crate::domain::gallery_metadata_filter::{
    GalleryDateRange, GalleryMetadataFilter, GalleryNumberRange,
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyFilter {
    extension: String,
    bytes: GalleryNumberRange,
    width: GalleryNumberRange,
    height: GalleryNumberRange,
    date: GalleryDateRange,
    camera_text: String,
}

pub(super) const VERSION: i64 = 2;
pub(super) const LEGACY_EMPTY: &str = r#"{"extension":"","bytes":{"minimum":null,"maximum":null},"width":{"minimum":null,"maximum":null},"height":{"minimum":null,"maximum":null},"date":{"field":"Capture","start":null,"end":null},"camera_text":""}"#;

pub(super) fn encode(filter: &GalleryMetadataFilter) -> Result<String, ScanError> {
    filter.validate()?;
    serde_json::to_string(filter).map_err(|_| invalid())
}

pub(super) fn decode(payload: &str, version: i64) -> Result<GalleryMetadataFilter, ScanError> {
    if payload.len() > 4096 {
        return Err(invalid());
    }
    let filter: GalleryMetadataFilter = match version {
        1 => {
            let legacy: LegacyFilter = serde_json::from_str(payload).map_err(|_| invalid())?;
            GalleryMetadataFilter {
                extensions: if legacy.extension.is_empty() {
                    Vec::new()
                } else {
                    vec![legacy.extension]
                },
                bytes: legacy.bytes,
                width: legacy.width,
                height: legacy.height,
                date: legacy.date,
                camera_text: legacy.camera_text,
            }
        }
        VERSION => serde_json::from_str(payload).map_err(|_| invalid())?,
        _ => return Err(invalid()),
    };
    filter.validate()?;
    Ok(filter)
}

fn invalid() -> ScanError {
    ScanError::new(
        "catalog_review_filter_invalid",
        "The saved organization scope has invalid metadata filters",
    )
}

#[cfg(test)]
pub(super) fn encode_legacy(filter: &GalleryMetadataFilter) -> String {
    assert!(filter.extensions.len() <= 1);
    let mut value = serde_json::to_value(filter).unwrap();
    let object = value.as_object_mut().unwrap();
    object.remove("extensions");
    object.insert(
        "extension".into(),
        serde_json::json!(filter.extensions.first().cloned().unwrap_or_default()),
    );
    serde_json::to_string(&value).unwrap()
}
