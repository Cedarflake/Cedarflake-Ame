use rusqlite::{params_from_iter, types::Value};

use crate::domain::ScanError;

use super::super::{SqliteCatalog, database_error};

pub(crate) struct RetainedScanIssue {
    pub(crate) id: i64,
    pub(crate) path: Option<String>,
}

pub(crate) fn load_retained_scan_issue_window(
    catalog: &SqliteCatalog,
    scan_id: &str,
    codes: &[&str],
    after_id: i64,
    limit: u32,
) -> Result<Vec<RetainedScanIssue>, ScanError> {
    if limit == 0 || limit > 256 || codes.is_empty() || codes.len() > 32 {
        return Err(invalid_filter());
    }
    let placeholders = (4..4 + codes.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut parameters = vec![
        Value::Text(scan_id.to_owned()),
        Value::Integer(after_id),
        Value::Integer(i64::from(limit)),
    ];
    parameters.extend(codes.iter().map(|code| Value::Text((*code).to_owned())));
    let mut statement = catalog
        .connection
        .prepare(&format!(
            "SELECT id, path FROM scan_issues
         WHERE scan_id = ?1 AND id > ?2 AND code IN ({placeholders}) ORDER BY id LIMIT ?3"
        ))
        .map_err(database_error)?;
    statement
        .query_map(params_from_iter(parameters), |row| {
            Ok(RetainedScanIssue {
                id: row.get(0)?,
                path: row.get(1)?,
            })
        })
        .map_err(database_error)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(database_error)
}

pub(crate) fn retained_scan_has_unclassified_issues(
    catalog: &SqliteCatalog,
    scan_id: &str,
    classified_codes: &[&str],
) -> Result<bool, ScanError> {
    if classified_codes.is_empty() || classified_codes.len() > 32 {
        return Err(invalid_filter());
    }
    let placeholders = (2..2 + classified_codes.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let mut parameters = vec![Value::Text(scan_id.to_owned())];
    parameters.extend(
        classified_codes
            .iter()
            .map(|code| Value::Text((*code).to_owned())),
    );
    catalog.connection.query_row(&format!(
        "SELECT EXISTS(SELECT 1 FROM scan_issues WHERE scan_id = ?1 AND code NOT IN ({placeholders}))"
    ), params_from_iter(parameters), |row| row.get(0)).map_err(database_error)
}

fn invalid_filter() -> ScanError {
    ScanError::new(
        "catalog_retained_issue_filter_invalid",
        "Retained issue recovery requires a bounded nonempty code filter and window",
    )
}
