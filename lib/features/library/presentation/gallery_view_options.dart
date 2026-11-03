import "../domain/library_view_preferences.dart";

export "../domain/library_view_preferences.dart"
    show GalleryLayoutShape, GalleryThumbnailSize;

extension GalleryThumbnailSizeValue on GalleryThumbnailSize {
  double get targetExtent => switch (this) {
    GalleryThumbnailSize.small => 128,
    GalleryThumbnailSize.medium => 192,
    GalleryThumbnailSize.large => 256,
  };
}
