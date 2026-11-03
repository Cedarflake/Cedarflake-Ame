use std::collections::BTreeSet;

use rusqlite::types::Value;

const MAX_INDEXED_TERMS: usize = 64;

pub(super) fn push_text_filter(text: &str, clauses: &mut Vec<String>, parameters: &mut Vec<Value>) {
    let search = text.trim();
    if search.is_empty() {
        return;
    }
    if let Some(candidates) = candidate_terms(search) {
        clauses.push(
            "locations.gallery_search_id IN (
               SELECT rowid
               FROM gallery_search_terms
               WHERE gallery_search_terms MATCH ?
             )"
            .to_owned(),
        );
        parameters.push(Value::Text(candidates));
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

fn candidate_terms(search: &str) -> Option<String> {
    // Trigrams cannot match shorter strings. NUL also terminates FTS tokenization.
    let character_count = search.chars().count();
    if character_count < 3 || search.contains('\0') {
        return None;
    }
    let term_count = character_count - 2;
    let stride = term_count.div_ceil(MAX_INDEXED_TERMS - 1);
    let mut characters = search.chars();
    let mut previous = [characters.next()?, characters.next()?];
    let mut terms = BTreeSet::new();
    for (index, last) in characters.enumerate() {
        if index % stride == 0 || index + 1 == term_count {
            let term: String = [previous[0], previous[1], last].into_iter().collect();
            terms.insert(format!("\"{}\"", term.replace('"', "\"\"")));
        }
        previous = [previous[1], last];
    }
    Some(terms.into_iter().collect::<Vec<_>>().join(" AND "))
}
