import "../../../src/rust/domain.dart" as rust;
import "../../../src/rust/domain/classification_review.dart" as classification;
import "../domain/library_gallery_collection.dart";

rust.GalleryCollection mapLibraryCollection(LibraryGalleryCollection value) =>
    switch (value) {
      LibraryAllCollection() => const rust.GalleryCollection.library_(),
      LibraryFavoritesCollection() => const rust.GalleryCollection.favorites(),
      LibraryAlbumCollection(:final albumId) => rust.GalleryCollection.album(
        albumId: albumId,
      ),
      LibrarySmartAlbumCollection(:final categoryId, :final review) =>
        rust.GalleryCollection.smartAlbum(
          categoryId: categoryId,
          review: switch (review) {
            LibraryClassificationReviewFilter.all =>
              classification.ClassificationReviewFilter.all,
            LibraryClassificationReviewFilter.pending =>
              classification.ClassificationReviewFilter.pending,
            LibraryClassificationReviewFilter.confirmed =>
              classification.ClassificationReviewFilter.confirmed,
          },
        ),
    };

LibraryGalleryCollection mapRustCollection(rust.GalleryCollection value) =>
    switch (value) {
      rust.GalleryCollection_Library() => const LibraryAllCollection(),
      rust.GalleryCollection_Favorites() => const LibraryFavoritesCollection(),
      rust.GalleryCollection_Album(:final albumId) => LibraryAlbumCollection(
        albumId,
      ),
      rust.GalleryCollection_SmartAlbum(:final categoryId, :final review) =>
        LibrarySmartAlbumCollection(
          categoryId: categoryId,
          review: switch (review) {
            classification.ClassificationReviewFilter.all =>
              LibraryClassificationReviewFilter.all,
            classification.ClassificationReviewFilter.pending =>
              LibraryClassificationReviewFilter.pending,
            classification.ClassificationReviewFilter.confirmed =>
              LibraryClassificationReviewFilter.confirmed,
          },
        ),
    };
