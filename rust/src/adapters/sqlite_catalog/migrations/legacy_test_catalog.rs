use rusqlite::Connection;

use super::create_schema_v19;

pub(super) fn fresh_v19_catalog() -> Connection {
    let mut connection = Connection::open_in_memory().expect("catalog");
    let transaction = connection.transaction().expect("v19 transaction");
    create_schema_v19(&transaction).expect("fresh v19 schema");
    transaction.commit().expect("commit fresh v19 schema");
    connection
}
