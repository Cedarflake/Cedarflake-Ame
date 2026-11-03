pub(super) fn extend_expected(
    table: &str,
    version: i64,
    expected: &mut Vec<(&str, &str, bool, i64)>,
) {
    if table == "library_change_scan_handoff_items"
        && version >= 31
        && !expected
            .iter()
            .any(|column| column.0 == "source_revision_token")
    {
        expected.extend([
            ("source_revision_token", "TEXT", false, 0),
            ("source_generation", "INTEGER", false, 0),
        ]);
    }
    if version >= 62
        && matches!(
            table,
            "library_change_scan_handoff_items" | "library_change_catch_up_handoffs"
        )
        && !expected.iter().any(|column| column.0 == "camera_make")
    {
        expected.extend([
            ("camera_make", "TEXT", false, 0),
            ("camera_model", "TEXT", false, 0),
        ]);
    }
}
