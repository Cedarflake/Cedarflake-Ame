import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/src/rust/domain.dart" as rust_domain;
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "collection identity distinguishes albums favorites and the library",
    () {
      const queries = [
        LibraryGalleryQuery(),
        LibraryGalleryQuery(collection: LibraryGalleryCollection.favorites()),
        LibraryGalleryQuery(
          collection: LibraryGalleryCollection.album("first"),
        ),
        LibraryGalleryQuery(
          collection: LibraryGalleryCollection.album("second"),
        ),
      ];
      expect(queries.toSet(), hasLength(4));
      expect(
        queries[2].copyWith(searchText: "image").copyWith(searchText: ""),
        queries[2],
      );
      expect(
        queries[3].copyWith(
          collection: const LibraryGalleryCollection.album("first"),
        ),
        queries[2],
      );
      expect(queries[2].foldsDuplicates, isTrue);
      expect(queries[1].foldsDuplicates, isFalse);
      expect(
        queries[1]
            .copyWith(duplicateMode: LibraryGalleryDuplicateMode.merged)
            .foldsDuplicates,
        isTrue,
      );
    },
  );

  test("bridge retains collection source search and ordering in one query", () {
    const query = LibraryGalleryQuery(
      collection: LibraryGalleryCollection.album("album-id"),
      rootId: "root",
      folderRelativePath: "folder",
      includeDescendants: false,
      searchText: "image",
      sortKey: LibraryGallerySortKey.fileName,
      sortDirection: LibraryGallerySortDirection.ascending,
    );
    final mapped = mapLibraryGalleryQueryToRust(query);
    expect(
      mapped.collection,
      const rust_domain.GalleryCollection.album(albumId: "album-id"),
    );
    expect(mapped.rootId, "root");
    expect(mapped.folderRelativePath, "folder");
    expect(mapped.includeDescendants, isFalse);
    expect(mapped.searchText, "image");
    expect(mapped.sortKey, rust_domain.GallerySortKey.fileName);
    expect(mapped.sortDirection, rust_domain.GallerySortDirection.ascending);
    expect(
      mapLibraryGalleryQueryToRust(
        query.copyWith(collection: const LibraryGalleryCollection.favorites()),
      ).collection,
      const rust_domain.GalleryCollection.favorites(),
    );
    expect(
      mapLibraryGalleryQueryToRust(const LibraryGalleryQuery()).collection,
      const rust_domain.GalleryCollection.library_(),
    );
  });
}
