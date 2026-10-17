use rusqlite::{Connection, OptionalExtension, params};

use crate::domain::ScanError;

use super::database_error;

pub(super) fn schema_version(connection: &Connection) -> Result<i64, ScanError> {
    connection
        .query_row("SELECT version FROM schema_info LIMIT 1", [], |row| {
            row.get(0)
        })
        .map_err(database_error)
}

pub(super) fn table_columns_match(
    connection: &Connection,
    table: &str,
    expected: &[(&str, &str, bool, i64)],
) -> Result<bool, ScanError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info('{table}')"))
        .map_err(database_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, i64>(5)?,
            ))
        })
        .map_err(database_error)?;
    let actual = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    let mut expected = expected.to_vec();
    if schema_version(connection)? >= 31
        && !expected
            .iter()
            .any(|column| column.0 == "source_revision_token")
        && table == "library_change_scan_handoff_items"
    {
        expected.push(("source_revision_token", "TEXT", false, 0));
        expected.push(("source_generation", "INTEGER", false, 0));
    }
    Ok(actual.len() == expected.len()
        && actual.iter().zip(&expected).all(|(actual, expected)| {
            actual.0 == expected.0
                && actual.1.eq_ignore_ascii_case(expected.1)
                && actual.2 == expected.2
                && actual.3 == expected.3
        }))
}

pub(super) fn named_index_matches(
    connection: &Connection,
    table: &str,
    index: &str,
    expected_columns: &[&str],
) -> Result<bool, ScanError> {
    let index_is_plain = connection
        .query_row(
            &format!(
                "SELECT EXISTS(SELECT 1 FROM pragma_index_list('{table}')
                 WHERE name = ?1 AND \"unique\" = 0 AND partial = 0)"
            ),
            [index],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)?;
    if !index_is_plain {
        return Ok(false);
    }
    let mut statement = connection
        .prepare(&format!("PRAGMA index_info('{index}')"))
        .map_err(database_error)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(2))
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)?;
    Ok(columns == expected_columns)
}

pub(super) fn schema_object_sql_matches(
    connection: &Connection,
    object_type: &str,
    name: &str,
    expected: &str,
) -> Result<bool, ScanError> {
    let actual = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = ?1 AND name = ?2",
            params![object_type, name],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(database_error)?;
    Ok(
        actual
            .is_some_and(|actual| normalize_schema_sql(&actual) == normalize_schema_sql(expected)),
    )
}

pub(super) fn normalize_schema_sql(value: &str) -> String {
    let mut canonical = String::with_capacity(value.len());
    let mut characters = value.chars().peekable();
    let mut pending_whitespace = false;
    while let Some(character) = characters.next() {
        if character.is_whitespace() {
            pending_whitespace = true;
            continue;
        }
        if pending_whitespace
            && canonical
                .chars()
                .next_back()
                .is_some_and(|previous| schema_tokens_require_separator(previous, character))
        {
            canonical.push(' ');
        }
        pending_whitespace = false;
        if matches!(character, '\'' | '"' | '`' | '[') {
            canonical.push(character);
            let terminator = if character == '[' { ']' } else { character };
            while let Some(quoted) = characters.next() {
                canonical.push(quoted);
                if quoted != terminator {
                    continue;
                }
                if characters.peek() == Some(&terminator) {
                    canonical.push(terminator);
                    characters.next();
                } else {
                    break;
                }
            }
        } else {
            canonical.extend(character.to_lowercase());
        }
    }
    canonical
}

fn schema_tokens_require_separator(previous: char, next: char) -> bool {
    !is_schema_separator(previous) && !is_schema_separator(next)
}

fn is_schema_separator(character: char) -> bool {
    matches!(
        character,
        '(' | ')'
            | ','
            | ';'
            | '.'
            | '='
            | '<'
            | '>'
            | '!'
            | '+'
            | '-'
            | '*'
            | '/'
            | '%'
            | '|'
            | '&'
            | '~'
    )
}

pub(super) fn cascade_foreign_key_matches(
    connection: &Connection,
    table: &str,
    from: &str,
    target_table: &str,
    target_column: &str,
) -> Result<bool, ScanError> {
    connection
        .query_row(
            &format!(
                "SELECT COUNT(*) = 1 AND EXISTS(
                   SELECT 1 FROM pragma_foreign_key_list('{table}')
                   WHERE \"table\" = ?1 AND \"from\" = ?2 AND \"to\" = ?3
                     AND on_update = 'NO ACTION' AND on_delete = 'CASCADE'
                     AND \"match\" = 'NONE'
                 ) FROM pragma_foreign_key_list('{table}')"
            ),
            params![target_table, from, target_column],
            |row| row.get::<_, bool>(0),
        )
        .map_err(database_error)
}

#[cfg(test)]
mod tests;
