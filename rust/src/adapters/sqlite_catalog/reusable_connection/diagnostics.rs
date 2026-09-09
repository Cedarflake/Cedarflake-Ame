use super::*;

pub(super) fn read_journal_mode(connection: &Connection) -> Result<String, ScanError> {
    let mut statement = measure("proof_journal_mode_prepare", || {
        connection.prepare("PRAGMA journal_mode")
    })
    .map_err(database_error)?;
    let result = measure("proof_journal_mode_execute_and_reset", || {
        statement.query_row([], |row| row.get::<_, String>(0))
    });
    measure("proof_journal_mode_statement_drop", || drop(statement));
    result.map_err(database_error)
}
