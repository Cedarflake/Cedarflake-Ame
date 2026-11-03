use rusqlite::Connection;

pub(super) fn print(connection: &Connection) {
    let (allocated, free): (i64, i64) = connection
        .query_row(
            "SELECT page_count * page_size, freelist_count * page_size
             FROM pragma_page_count(), pragma_freelist_count(), pragma_page_size()",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("catalog page accounting");
    println!("AME_SYNTHETIC_CATALOG_PAGES allocated_bytes={allocated} free_bytes={free}");
    let mut statement = connection
        .prepare(
            "SELECT name, SUM(pgsize), SUM(unused) FROM dbstat
             GROUP BY name HAVING SUM(pgsize) >= 65536 ORDER BY SUM(pgsize) DESC",
        )
        .expect("catalog object accounting");
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .expect("catalog object pages");
    for row in rows {
        let (name, bytes, unused) = row.expect("catalog object size");
        println!("AME_SYNTHETIC_CATALOG_OBJECT name={name} bytes={bytes} unused_bytes={unused}");
    }
}
