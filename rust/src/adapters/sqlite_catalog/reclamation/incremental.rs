use rusqlite::Connection;

pub(super) fn reclaim_pages(connection: &Connection, max_pages: u32) -> rusqlite::Result<()> {
    let mut statement = connection.prepare(&format!("PRAGMA incremental_vacuum({max_pages})"))?;
    let mut rows = statement.query([])?;
    // SQLite advances incremental vacuum as its result rows are stepped. Stopping after
    // the first row releases only one page, regardless of the admitted batch size.
    while rows.next()?.is_some() {}
    Ok(())
}
