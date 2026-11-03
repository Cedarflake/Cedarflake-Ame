use super::*;

#[test]
fn sqlite_cancellation_releases_the_read_transaction_and_progress_handler() {
    let connection = Connection::open_in_memory().unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let sql = "WITH RECURSIVE values_to_read(value) AS (
        SELECT 1 UNION ALL SELECT value + 1 FROM values_to_read WHERE value < 10000)
        SELECT SUM(value) FROM values_to_read";
    let error = read::<i64>(&connection, &cancelled, |connection| {
        cancelled.store(true, Ordering::Release);
        let interrupted = connection
            .query_row(sql, [], |row| row.get::<_, i64>(0))
            .unwrap_err();
        assert_eq!(
            interrupted.sqlite_error_code(),
            Some(rusqlite::ErrorCode::OperationInterrupted)
        );
        Err(database_error(interrupted))
    })
    .unwrap_err();
    assert_eq!(error.code, "classification_cancelled");
    assert!(connection.is_autocommit());
    assert_eq!(
        connection
            .query_row(sql, [], |row| row.get::<_, i64>(0))
            .unwrap(),
        50_005_000
    );
    cancelled.store(false, Ordering::Release);
    let error = read(&connection, &cancelled, |connection| {
        connection
            .execute_batch("SELECT missing_column")
            .map_err(database_error)
    })
    .unwrap_err();
    assert_ne!(error.code, "classification_cancelled");
    assert!(connection.is_autocommit());
}
