use super::super::super::metadata_filter_payload;
use super::{Cell, records};
use crate::domain::ScanError;

pub(super) fn fields(kind: &records::RecordKind, version: u32) -> &[records::Field] {
    let omitted = match (version, kind.name) {
        (1, "version_evidence") => 1,
        (1 | 2, "review_scope") => 3,
        (3, "review_scope") => 1,
        (1 | 2, "bookmark") => 3,
        _ => 0,
    };
    &kind.fields[..kind.fields.len() - omitted]
}

pub(super) fn upgrade(kind: &str, values: &mut Vec<Cell>, version: u32) -> Result<(), ScanError> {
    match (version, kind) {
        (1, "version_evidence") => values.push(Cell::Integer(0)),
        (1 | 2, "review_scope") => values.extend([
            Cell::Text("library_admission".into()),
            Cell::Text("descending".into()),
        ]),
        (1 | 2, "bookmark") => {
            if let Cell::Integer(admission) = values[1] {
                values.extend([
                    Cell::Integer(0),
                    Cell::Text(String::new()),
                    Cell::Integer(admission),
                ]);
            }
        }
        _ => {}
    }
    if version < 4 && kind == "review_scope" {
        values.push(Cell::Text(metadata_filter_payload::LEGACY_EMPTY.into()));
    }
    if version < 7 && kind == "review_scope" {
        let Some(Cell::Text(payload)) = values.get_mut(9) else {
            return Err(super::invalid());
        };
        *payload = metadata_filter_payload::decode(payload, 1)
            .and_then(|filter| metadata_filter_payload::encode(&filter))
            .map_err(|_| super::super::error("records_invalid", "Invalid saved metadata filter"))?;
    }
    Ok(())
}
