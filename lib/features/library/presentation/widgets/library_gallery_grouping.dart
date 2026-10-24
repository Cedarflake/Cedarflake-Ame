import "../../domain/library_models.dart";

enum LibraryGalleryGrouping {
  date,
  continuous;

  factory LibraryGalleryGrouping.forQuery(LibraryGalleryQuery query) =>
      query.isChronological ? date : continuous;

  factory LibraryGalleryGrouping.forSortKey(LibraryGallerySortKey sortKey) =>
      sortKey == LibraryGallerySortKey.fileName ? continuous : date;
}
