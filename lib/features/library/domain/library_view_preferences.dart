import "library_models.dart";

enum GalleryLayoutShape { equalHeight, square }

enum GalleryThumbnailSize { small, medium, large }

class LibraryViewPreferences {
  const LibraryViewPreferences({
    this.layoutShape = GalleryLayoutShape.equalHeight,
    this.thumbnailSize = GalleryThumbnailSize.medium,
    this.sortKey = LibraryGallerySortKey.captureTime,
    this.sortDirection = LibraryGallerySortDirection.descending,
    this.duplicateMode = LibraryGalleryDuplicateMode.merged,
  });

  final GalleryLayoutShape layoutShape;
  final GalleryThumbnailSize thumbnailSize;
  final LibraryGallerySortKey sortKey;
  final LibraryGallerySortDirection sortDirection;
  final LibraryGalleryDuplicateMode duplicateMode;

  LibraryViewPreferences copyWith({
    GalleryLayoutShape? layoutShape,
    GalleryThumbnailSize? thumbnailSize,
    LibraryGallerySortKey? sortKey,
    LibraryGallerySortDirection? sortDirection,
    LibraryGalleryDuplicateMode? duplicateMode,
  }) {
    return LibraryViewPreferences(
      layoutShape: layoutShape ?? this.layoutShape,
      thumbnailSize: thumbnailSize ?? this.thumbnailSize,
      sortKey: sortKey ?? this.sortKey,
      sortDirection: sortDirection ?? this.sortDirection,
      duplicateMode: duplicateMode ?? this.duplicateMode,
    );
  }
}
