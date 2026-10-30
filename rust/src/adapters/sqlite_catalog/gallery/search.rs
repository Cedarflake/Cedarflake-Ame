use rusqlite::types::Value;

pub(super) fn push_text_filter(text: &str, clauses: &mut Vec<String>, parameters: &mut Vec<Value>) {
    let search = text.trim();
    if search.is_empty() {
        return;
    }
    // Trigrams cannot match shorter strings. NUL also terminates FTS tokenization.
    if search.chars().take(3).count() == 3 && !search.contains('\0') {
        clauses.push(
            "(locations.scan_id, locations.location_id) IN (
               SELECT documents.scan_id, documents.location_id
               FROM gallery_search_terms
               JOIN gallery_search_documents AS documents ON documents.id = gallery_search_terms.rowid
               WHERE gallery_search_terms MATCH ?
             )"
            .to_owned(),
        );
        parameters.push(Value::Text(format!("\"{}\"", search.replace('"', "\"\""))));
    }
    // Preserve literal substring and SQLite's existing case rules after FTS candidate reduction.
    clauses.push(
        "(instr(lower(locations.absolute_path), lower(?)) > 0 OR \
         instr(lower(locations.relative_path), lower(?)) > 0)"
            .to_owned(),
    );
    parameters.extend([
        Value::Text(search.to_owned()),
        Value::Text(search.to_owned()),
    ]);
}
