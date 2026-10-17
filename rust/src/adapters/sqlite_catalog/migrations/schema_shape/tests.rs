use super::*;

fn catalog(version: i64) -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    connection
        .execute_batch("CREATE TABLE schema_info (version INTEGER NOT NULL);")
        .unwrap();
    connection
        .execute("INSERT INTO schema_info VALUES (?1)", [version])
        .unwrap();
    connection
}

#[test]
fn table_shape_preserves_column_order_nullability_and_primary_key() {
    let connection = catalog(30);
    connection
        .execute_batch("CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT NOT NULL);")
        .unwrap();
    let expected = [("id", "integer", false, 1), ("name", "text", true, 0)];
    assert!(table_columns_match(&connection, "sample", &expected).unwrap());
    for rejected in [
        [("name", "text", true, 0), ("id", "integer", false, 1)],
        [("id", "text", false, 1), ("name", "text", true, 0)],
        [("id", "integer", false, 1), ("name", "text", false, 0)],
        [("id", "integer", false, 0), ("name", "text", true, 0)],
    ] {
        assert!(!table_columns_match(&connection, "sample", &rejected).unwrap());
    }
    connection
        .execute_batch("ALTER TABLE sample ADD COLUMN extra TEXT;")
        .unwrap();
    assert!(!table_columns_match(&connection, "sample", &expected).unwrap());
}

#[test]
fn source_revision_handoff_columns_are_required_only_from_version_31() {
    let connection = catalog(30);
    connection
        .execute_batch("CREATE TABLE library_change_scan_handoff_items (id INTEGER PRIMARY KEY);")
        .unwrap();
    let original = [("id", "INTEGER", false, 1)];
    assert!(
        table_columns_match(&connection, "library_change_scan_handoff_items", &original).unwrap()
    );
    connection
        .execute_batch("UPDATE schema_info SET version = 31;")
        .unwrap();
    assert!(
        !table_columns_match(&connection, "library_change_scan_handoff_items", &original).unwrap()
    );
    connection
        .execute_batch(
            "ALTER TABLE library_change_scan_handoff_items ADD COLUMN source_revision_token TEXT;
             ALTER TABLE library_change_scan_handoff_items ADD COLUMN source_generation INTEGER;",
        )
        .unwrap();
    assert!(
        table_columns_match(&connection, "library_change_scan_handoff_items", &original).unwrap()
    );
    let explicit = [
        ("id", "INTEGER", false, 1),
        ("source_revision_token", "TEXT", false, 0),
        ("source_generation", "INTEGER", false, 0),
    ];
    assert!(
        table_columns_match(&connection, "library_change_scan_handoff_items", &explicit).unwrap()
    );
}

#[test]
fn index_proof_requires_plain_index_and_exact_column_order() {
    let connection = catalog(30);
    connection
        .execute_batch(
            "CREATE TABLE sample (id INTEGER PRIMARY KEY, name TEXT);
             CREATE INDEX plain ON sample(name, id);
             CREATE UNIQUE INDEX unique_name ON sample(name);
             CREATE INDEX partial_name ON sample(name) WHERE name IS NOT NULL;",
        )
        .unwrap();
    assert!(named_index_matches(&connection, "sample", "plain", &["name", "id"]).unwrap());
    assert!(!named_index_matches(&connection, "sample", "plain", &["id", "name"]).unwrap());
    for name in ["unique_name", "partial_name", "missing"] {
        assert!(!named_index_matches(&connection, "sample", name, &["name"]).unwrap());
    }
}

#[test]
fn foreign_key_proof_requires_one_exact_cascade_relationship() {
    let connection = catalog(30);
    connection
        .execute_batch(
            "CREATE TABLE parent (id INTEGER PRIMARY KEY);
             CREATE TABLE child (parent_id INTEGER REFERENCES parent(id) ON DELETE CASCADE);
             CREATE TABLE no_cascade (parent_id INTEGER REFERENCES parent(id));
             CREATE TABLE extra_key (
                 parent_id INTEGER REFERENCES parent(id) ON DELETE CASCADE,
                 other_id INTEGER REFERENCES parent(id) ON DELETE CASCADE
             );",
        )
        .unwrap();
    assert!(
        cascade_foreign_key_matches(&connection, "child", "parent_id", "parent", "id").unwrap()
    );
    for table in ["no_cascade", "extra_key", "missing"] {
        assert!(
            !cascade_foreign_key_matches(&connection, table, "parent_id", "parent", "id").unwrap()
        );
    }
    assert!(
        !cascade_foreign_key_matches(&connection, "child", "parent_id", "other", "id").unwrap()
    );
}

#[test]
fn schema_object_proof_ignores_formatting_but_preserves_literal_values() {
    let connection = catalog(30);
    connection
        .execute_batch("CREATE TABLE sample (name TEXT CHECK(name = 'A B'));")
        .unwrap();
    let expected = "create table sample(name text check(name='A B'))";
    assert!(schema_object_sql_matches(&connection, "table", "sample", expected).unwrap());
    assert!(!schema_object_sql_matches(&connection, "table", "missing", expected).unwrap());
    assert!(
        !schema_object_sql_matches(
            &connection,
            "table",
            "sample",
            &expected.replace("A B", "a b")
        )
        .unwrap()
    );
}

#[test]
fn schema_sql_canonicalization_preserves_quoted_tokens_and_escapes() {
    let formatted = "CREATE  TABLE [Range Name] (kind TEXT CHECK(kind = 'a  b'),\n\
                     note TEXT CHECK(note = 'it''s'), value BLOB DEFAULT X'0A')";
    let compact = "create table [Range Name](kind text check(kind='a  b'),\
                   note text check(note='it''s'),value blob default x'0A')";
    assert_eq!(
        normalize_schema_sql(formatted),
        normalize_schema_sql(compact)
    );
    for changed in [
        compact.replace("[Range Name]", "[range name]"),
        compact.replace("'a  b'", "'a b'"),
        compact.replace("'it''s'", "'IT''S'"),
        compact.replace("x'0A'", "x'0a'"),
    ] {
        assert_ne!(
            normalize_schema_sql(compact),
            normalize_schema_sql(&changed)
        );
    }
    assert_ne!(
        normalize_schema_sql("CREATE TABLE `Range` (`Value` TEXT)"),
        normalize_schema_sql("CREATE TABLE `range` (`Value` TEXT)")
    );
    assert_ne!(
        normalize_schema_sql("CREATE TABLE \"Range\" (\"Value\" TEXT)"),
        normalize_schema_sql("CREATE TABLE \"range\" (\"Value\" TEXT)")
    );
}
