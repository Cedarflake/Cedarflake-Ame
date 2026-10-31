import "package:cedarflake_ame/features/library/application/library_gallery_collection_mapping.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("smart-album category and review scope survive native translation", () {
    final collections = <LibraryGalleryCollection>{};
    for (final category in [null, "drawing", "photo"]) {
      for (final review in LibraryClassificationReviewFilter.values) {
        final collection = LibrarySmartAlbumCollection(
          categoryId: category,
          review: review,
        );
        expect(mapRustCollection(mapLibraryCollection(collection)), collection);
        expect(collections.add(collection), isTrue);
        expect(
          LibraryGalleryQuery(collection: collection).foldsDuplicates,
          isTrue,
        );
      }
    }
    expect(collections, hasLength(9));
    for (final collection in <LibraryGalleryCollection>[
      const LibraryAllCollection(),
      const LibraryFavoritesCollection(),
      const LibraryAlbumCollection("manual"),
    ]) {
      expect(mapRustCollection(mapLibraryCollection(collection)), collection);
      expect(collections.contains(collection), isFalse);
    }
  });
}
