use std::path::Path;

use super::direct_disk_namespace;

#[test]
fn only_identical_direct_disk_namespaces_admit_the_pinned_file() {
    let canonical = Path::new(r"\\?\C:\catalog\ame.sqlite3");
    assert!(direct_disk_namespace(
        Path::new(r"C:\catalog\ame.sqlite3"),
        canonical,
    ));
    assert!(direct_disk_namespace(canonical, canonical));
    for requested in [
        r"C:\alias\ame.sqlite3",
        r"C:\catalog-b\ame.sqlite3",
        r"C:\CATALOG\ame.sqlite3",
        r"C:\catalog\.\ame.sqlite3",
        r"C:\catalog\child\..\ame.sqlite3",
        r"C:\catalog\ame.sqlite3.",
        "C:\\catalog\\ame.sqlite3 ",
        r"C:\catalog\\ame.sqlite3",
        r"C:/catalog/ame.sqlite3",
        r"C:catalog\ame.sqlite3",
        r"catalog\ame.sqlite3",
        r"\\server\share\ame.sqlite3",
        r"\\?\UNC\server\share\ame.sqlite3",
        r"\\?\Volume{1234}\catalog\ame.sqlite3",
    ] {
        assert!(
            !direct_disk_namespace(Path::new(requested), canonical),
            "must resolve the current path independently: {requested}",
        );
    }
}
