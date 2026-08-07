enum GalleryLayoutShape { equalHeight, square }

enum GalleryThumbnailSize { small, medium, large }

extension GalleryThumbnailSizeValue on GalleryThumbnailSize {
  double get targetExtent => switch (this) {
    GalleryThumbnailSize.small => 96,
    GalleryThumbnailSize.medium => 138,
    GalleryThumbnailSize.large => 190,
  };
}
