use rusqlite::types::Value;

use crate::domain::gallery_metadata_filter::{
    GalleryDateField, GalleryMetadataFilter, GalleryNumberRange,
};

pub(super) fn push_filters(
    filter: &GalleryMetadataFilter,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    if !filter.extension.is_empty() {
        clauses.push(
            "lower(substr(locations.relative_path, -length(?) - 1)) = '.' || lower(?)".into(),
        );
        parameters.extend([
            Value::Text(filter.extension.clone()),
            Value::Text(filter.extension.clone()),
        ]);
    }
    for (column, range) in [
        ("file_size", &filter.bytes),
        ("width", &filter.width),
        ("height", &filter.height),
    ] {
        push_number_range(column, range, clauses, parameters);
    }
    let date = match filter.date.field {
        GalleryDateField::Capture => "substr(locations.capture_local_time, 1, 10)",
        GalleryDateField::Created => {
            "date(locations.created_unix_ms / 1000, 'unixepoch', 'localtime')"
        }
        GalleryDateField::Modified => {
            "date(locations.modified_unix_ms / 1000, 'unixepoch', 'localtime')"
        }
    };
    for (operator, value) in [(">=", &filter.date.start), ("<=", &filter.date.end)] {
        if let Some(value) = value {
            clauses.push(format!("{date} {operator} ?"));
            parameters.push(Value::Text(value.clone()));
        }
    }
    if !filter.camera_text.is_empty() {
        clauses.push("instr(lower(COALESCE(locations.camera_make, '') || ' ' || COALESCE(locations.camera_model, '')), lower(?)) > 0".into());
        parameters.push(Value::Text(filter.camera_text.clone()));
    }
}

fn push_number_range(
    column: &str,
    range: &GalleryNumberRange,
    clauses: &mut Vec<String>,
    parameters: &mut Vec<Value>,
) {
    if column != "file_size" && (range.minimum.is_some() || range.maximum.is_some()) {
        clauses.push(format!("locations.{column} > 0"));
    }
    for (operator, value) in [(">=", range.minimum), ("<=", range.maximum)] {
        if let Some(value) = value {
            clauses.push(format!("locations.{column} {operator} ?"));
            parameters.push(Value::Integer(value as i64));
        }
    }
}
