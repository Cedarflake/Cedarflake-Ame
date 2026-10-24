sealed class LibraryGalleryCollection {
  const LibraryGalleryCollection();

  const factory LibraryGalleryCollection.library() = LibraryAllCollection;
  const factory LibraryGalleryCollection.favorites() =
      LibraryFavoritesCollection;
  const factory LibraryGalleryCollection.album(String albumId) =
      LibraryAlbumCollection;

  @override
  bool operator ==(Object other) => switch ((this, other)) {
    (LibraryAllCollection(), LibraryAllCollection()) => true,
    (LibraryFavoritesCollection(), LibraryFavoritesCollection()) => true,
    (
      LibraryAlbumCollection(:final albumId),
      LibraryAlbumCollection(albumId: final otherId),
    ) =>
      albumId == otherId,
    _ => false,
  };

  @override
  int get hashCode => switch (this) {
    LibraryAllCollection() => runtimeType.hashCode,
    LibraryFavoritesCollection() => runtimeType.hashCode,
    LibraryAlbumCollection(:final albumId) => Object.hash(runtimeType, albumId),
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
