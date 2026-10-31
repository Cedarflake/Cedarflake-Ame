sealed class LibraryGalleryCollection {
  const LibraryGalleryCollection();

  const factory LibraryGalleryCollection.library() = LibraryAllCollection;
  const factory LibraryGalleryCollection.favorites() =
      LibraryFavoritesCollection;
  const factory LibraryGalleryCollection.album(String albumId) =
      LibraryAlbumCollection;
  const factory LibraryGalleryCollection.smartAlbum({
    String? categoryId,
    LibraryClassificationReviewFilter review,
  }) = LibrarySmartAlbumCollection;

  @override
  bool operator ==(Object other) => switch ((this, other)) {
    (LibraryAllCollection(), LibraryAllCollection()) => true,
    (LibraryFavoritesCollection(), LibraryFavoritesCollection()) => true,
    (
      LibraryAlbumCollection(:final albumId),
      LibraryAlbumCollection(albumId: final otherId),
    ) =>
      albumId == otherId,
    (
      LibrarySmartAlbumCollection(:final categoryId, :final review),
      LibrarySmartAlbumCollection(
        categoryId: final otherId,
        review: final otherReview,
      ),
    ) =>
      categoryId == otherId && review == otherReview,
    _ => false,
  };

  @override
  int get hashCode => switch (this) {
    LibraryAllCollection() => runtimeType.hashCode,
    LibraryFavoritesCollection() => runtimeType.hashCode,
    LibraryAlbumCollection(:final albumId) => Object.hash(runtimeType, albumId),
    LibrarySmartAlbumCollection(:final categoryId, :final review) =>
      Object.hash(runtimeType, categoryId, review),
  };
}

final class LibraryAllCollection extends LibraryGalleryCollection {
  const LibraryAllCollection();
}

final class LibraryFavoritesCollection extends LibraryGalleryCollection {
  const LibraryFavoritesCollection();
}

final class LibraryAlbumCollection extends LibraryGalleryCollection {
  const LibraryAlbumCollection(this.albumId);

  final String albumId;
}

enum LibraryClassificationReviewFilter { all, pending, confirmed }

final class LibrarySmartAlbumCollection extends LibraryGalleryCollection {
  const LibrarySmartAlbumCollection({
    this.categoryId,
    this.review = LibraryClassificationReviewFilter.all,
  });

  final String? categoryId;
  final LibraryClassificationReviewFilter review;
}
