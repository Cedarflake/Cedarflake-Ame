import "../../domain/library_models.dart";

enum LibraryGalleryGrouping {
  date,
  continuous;

  factory LibraryGalleryGrouping.forQuery(LibraryGalleryQuery query) =>
      query.isChronological ? date : continuous;

  factory LibraryGalleryGrouping.forSortKey(LibraryGallerySortKey sortKey) =>
      switch (sortKey) {
        LibraryGallerySortKey.fileName ||
        LibraryGallerySortKey.libraryAdmission => continuous,
        _ => date,
      };
}
