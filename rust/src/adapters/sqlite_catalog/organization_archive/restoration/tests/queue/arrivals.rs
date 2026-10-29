use crate::domain::gallery_scroll_section::GalleryScrollSectionKind;
use crate::ports::GalleryQueryRepository;

use super::*;

fn ranges(
    catalog: &mut SqliteCatalog,
    query: &GalleryQuery,
) -> Vec<(GalleryScrollSectionKind, u64)> {
    catalog
        .load_query_snapshot(20, query, "arrival-restore", None)
        .unwrap()
        .timeline
        .scroll_sections
        .into_iter()
        .map(|section| (section.kind, section.item_count))
        .collect()
}

#[test]
fn restoration_preserves_an_open_modes_boundary_and_genuine_new_arrivals() {
    let (directory, source) = source();
    let path = directory.path().join("target.sqlite3");
    let mut target = SqliteCatalog::open(path.clone()).unwrap();
    import(&mut target, "old", 90);
    let boundary = target.load_organization_admission_boundary().unwrap();
    let query = GalleryQuery {
        organization_queue: Some(OrganizationQueue::Pending),
        sort_key: GallerySortKey::LibraryAdmission,
        new_since_admission: Some(boundary),
        ..GalleryQuery::default()
    };
    import(&mut target, "new", 91);
    let expected = vec![
        (GalleryScrollSectionKind::NewArrivals, 1),
        (GalleryScrollSectionKind::Earlier, 1),
    ];
    assert_eq!(ranges(&mut target, &query), expected);
    let high_water = target.load_organization_admission_boundary().unwrap();
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    for _ in 0..2 {
        apply(&target, &archive).unwrap();
        assert_eq!(
            target.load_organization_admission_boundary().unwrap(),
            high_water
        );
        assert_eq!(ranges(&mut target, &query), expected);
    }
    detach(&target, staged);
    drop(target);
    let mut target = SqliteCatalog::open(path).unwrap();
    assert_eq!(ranges(&mut target, &query), expected);
    import(&mut target, "newer", 92);
    assert_eq!(
        ranges(&mut target, &query),
        vec![
            (GalleryScrollSectionKind::NewArrivals, 2),
            (GalleryScrollSectionKind::Earlier, 1)
        ]
    );
    let next_entry = GalleryQuery {
        new_since_admission: Some(target.load_organization_admission_boundary().unwrap()),
        ..query.clone()
    };
    assert_eq!(
        ranges(&mut target, &next_entry),
        vec![(GalleryScrollSectionKind::Earlier, 3)]
    );
    let scoped = GalleryQuery {
        root_id: Some("current-root-old".into()),
        ..query
    };
    assert_eq!(
        ranges(&mut target, &scoped),
        vec![(GalleryScrollSectionKind::Earlier, 1)]
    );
}

#[test]
fn a_second_prefix_shift_preserves_prior_entry_coordinates() {
    let (directory, source) = source();
    let mut target = SqliteCatalog::open(directory.path().join("target.sqlite3")).unwrap();
    import(&mut target, "old", 90);
    let query = GalleryQuery {
        organization_queue: Some(OrganizationQueue::Pending),
        sort_key: GallerySortKey::LibraryAdmission,
        new_since_admission: Some(target.load_organization_admission_boundary().unwrap()),
        ..GalleryQuery::default()
    };
    let (archive, staged) = attach_archive(&target, &source, directory.path());
    apply(&target, &archive).unwrap();
    import(&mut target, "c", 3);
    target.unregister_root("current-root-c").unwrap();
    import(&mut target, "new", 91);
    let high_water = target.load_organization_admission_boundary().unwrap();
    apply(&target, &archive).unwrap();
    assert_eq!(
        target.load_organization_admission_boundary().unwrap(),
        high_water
    );
    assert_eq!(
        ranges(&mut target, &query),
        vec![
            (GalleryScrollSectionKind::NewArrivals, 1),
            (GalleryScrollSectionKind::Earlier, 1)
        ]
    );
    detach(&target, staged);
}
