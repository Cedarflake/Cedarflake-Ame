use super::{Cell, records};

pub(super) fn fields(kind: &records::RecordKind, version: u32) -> &[records::Field] {
    let omitted = match (version, kind.name) {
        (1, "version_evidence") => 1,
        (1 | 2, "review_scope") => 2,
        (1 | 2, "bookmark") => 3,
        _ => 0,
    };
    &kind.fields[..kind.fields.len() - omitted]
}

pub(super) fn upgrade(kind: &str, values: &mut Vec<Cell>, version: u32) {
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
}
