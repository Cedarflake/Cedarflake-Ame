use exif::{Exif, In, Tag, Value};

use crate::domain::{CameraMetadata, ScanIssue, media_metadata::MAX_CAMERA_FIELD_BYTES};

use super::metadata_issue;

pub(super) fn extract(
    exif: &Exif,
    source_path: &str,
    issues: &mut Vec<ScanIssue>,
) -> CameraMetadata {
    let mut read = |tag| match text_field(exif, tag) {
        Ok(value) => value,
        Err(message) => {
            issues.push(metadata_issue(
                source_path,
                "camera_metadata_invalid",
                message,
            ));
            None
        }
    };
    CameraMetadata {
        make: read(Tag::Make),
        model: read(Tag::Model),
    }
}

fn text_field(exif: &Exif, tag: Tag) -> Result<Option<String>, String> {
    let Some(field) = exif.get_field(tag, In::PRIMARY) else {
        return Ok(None);
    };
    let Value::Ascii(values) = &field.value else {
        return Err(format!("{tag} is not a text field"));
    };
    let [bytes] = values.as_slice() else {
        return Err(format!("{tag} must contain one text value"));
    };
    if bytes.len() > MAX_CAMERA_FIELD_BYTES {
        return Err(format!(
            "{tag} exceeds the {MAX_CAMERA_FIELD_BYTES}-byte evidence limit"
        ));
    }
    let value = std::str::from_utf8(bytes).map_err(|_| format!("{tag} is not valid UTF-8 text"))?;
    if value.chars().any(char::is_control) {
        return Err(format!("{tag} contains control characters"));
    }
    let value = value.trim();
    Ok((!value.is_empty()).then(|| value.to_owned()))
}

#[cfg(test)]
mod tests;
