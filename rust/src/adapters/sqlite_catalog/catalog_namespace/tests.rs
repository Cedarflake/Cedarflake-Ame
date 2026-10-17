use std::path::Path;

use super::{differing_disk_prefixes, direct_disk_namespace};

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

#[test]
fn dos_name_lookup_stops_before_the_identically_spelled_catalog_suffix() {
    let (requested, pinned) = differing_disk_prefixes(
        Path::new(r"C:\fixture\SHORT~1\storage\nested\catalog.sqlite3"),
        Path::new(r"\\?\C:\fixture\Long parent\storage\nested\catalog.sqlite3"),
    )
    .unwrap();
    assert_eq!(requested, Path::new(r"C:\fixture\SHORT~1"));
    assert_eq!(pinned, Path::new(r"\\?\C:\fixture\Long parent"));
}

#[test]
fn differing_names_remain_inside_the_fresh_lookup_even_when_an_ancestor_also_differs() {
    for (requested, pinned, requested_prefix, pinned_prefix) in [
        (
            r"C:\SHORT~1\child\OTHER~1\catalog.sqlite3",
            r"\\?\C:\Long parent\child\Other parent\catalog.sqlite3",
            r"C:\SHORT~1\child\OTHER~1",
            r"\\?\C:\Long parent\child\Other parent",
        ),
        (
            r"C:\catalog\AME~1.SQL",
            r"\\?\C:\catalog\ame.sqlite3",
            r"C:\catalog\AME~1.SQL",
            r"\\?\C:\catalog\ame.sqlite3",
        ),
        (
            r"C:\catalog\Child\ame.sqlite3",
            r"\\?\C:\catalog\child\ame.sqlite3",
            r"C:\catalog\Child",
            r"\\?\C:\catalog\child",
        ),
        (
            r"C:\first-catalog\ame.sqlite3",
            r"\\?\C:\catalog\ame.sqlite3",
            r"C:\first-catalog",
            r"\\?\C:\catalog",
        ),
    ] {
        assert_eq!(
            differing_disk_prefixes(Path::new(requested), Path::new(pinned)),
            Some((Path::new(requested_prefix), Path::new(pinned_prefix))),
        );
    }
}

#[test]
fn ambiguous_or_non_disk_spellings_never_gain_prefix_admission() {
    let ordinary = Path::new(r"C:\catalog\ame.sqlite3");
    for ambiguous in [
        r"C:\catalog\.\ame.sqlite3",
        r"C:\catalog\child\..\ame.sqlite3",
        r"C:\catalog\\ame.sqlite3",
        r"C:\catalog\ame.sqlite3.",
        "C:\\catalog\\ame.sqlite3 ",
        "C:\\catalog\0ignored\\ame.sqlite3",
        r"C:/catalog/ame.sqlite3",
        r"catalog\ame.sqlite3",
        r"\\server\share\catalog\ame.sqlite3",
        r"\\?\UNC\server\share\catalog\ame.sqlite3",
        r"\\?\Volume{1234}\catalog\ame.sqlite3",
    ] {
        let ambiguous = Path::new(ambiguous);
        assert_eq!(differing_disk_prefixes(ambiguous, ordinary), None);
        assert_eq!(differing_disk_prefixes(ordinary, ambiguous), None);
    }
}
