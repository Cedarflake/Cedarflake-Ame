import "../application/library_view_preferences.dart";

export "../application/library_view_preferences.dart"
    show GalleryLayoutShape, GalleryThumbnailSize;

extension GalleryThumbnailSizeValue on GalleryThumbnailSize {
  double get targetExtent => switch (this) {
    GalleryThumbnailSize.small => 96,
    GalleryThumbnailSize.medium => 138,
    GalleryThumbnailSize.large => 190,
  };
}
