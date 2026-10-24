import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/src/rust/domain.dart" as rust_domain;
import "package:cedarflake_ame/src/rust/domain/organization_queue.dart"
    as rust_organization;
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "organization queues have distinct query identity and preserve ordinary preferences",
    () {
      const ordinary = LibraryGalleryQuery(
        collection: LibraryGalleryCollection.favorites(),
        rootId: "root",
        sortKey: LibraryGallerySortKey.createdTime,
        sortDirection: LibraryGallerySortDirection.ascending,
      );
      final queries = [
        ordinary,
        for (final queue in LibraryOrganizationQueue.values)
          ordinary.copyWith(organizationQueue: queue),
      ];
      expect(queries.toSet(), hasLength(4));
      for (final query in queries.skip(1)) {
        expect(query.foldsDuplicates, isTrue);
        expect(query.isChronological, isFalse);
        expect(
          query.copyWith(searchText: "image").organizationQueue,
          query.organizationQueue,
        );
        expect(query.copyWith(organizationQueue: null), ordinary);
      }
      expect(ordinary.isChronological, isTrue);
      expect(ordinary.foldsDuplicates, isFalse);
    },
  );

  test("bridge retains queue collection and physical scope together", () {
    for (final queue in LibraryOrganizationQueue.values) {
      final query = LibraryGalleryQuery(
        organizationQueue: queue,
        collection: const LibraryGalleryCollection.favorites(),
        rootId: "root",
        folderRelativePath: "folder",
        includeDescendants: false,
        searchText: "image",
      );
      final mapped = mapLibraryGalleryQueryToRust(query);
      expect(
        mapped.organizationQueue,
        rust_organization.OrganizationQueue.values[queue.index],
      );
      expect(
        mapped.collection,
        const rust_domain.GalleryCollection.favorites(),
      );
      expect(mapped.rootId, "root");
      expect(mapped.folderRelativePath, "folder");
      expect(mapped.includeDescendants, isFalse);
      expect(mapped.searchText, "image");
    }
    expect(
      mapLibraryGalleryQueryToRust(
        const LibraryGalleryQuery(),
      ).organizationQueue,
      isNull,
    );
  });

  test(
    "progress counts retain organized groups independently of the active queue",
    () {
      final mapped = mapRustOrganizationCounts(
        rust_organization.OrganizationCounts(
          total: BigInt.from(12),
          pending: BigInt.from(4),
          deferred_: BigInt.from(2),
          organized: BigInt.from(5),
          ignored: BigInt.one,
        ),
      );
      expect(mapped.total, 12);
      expect(mapped.organized, 5);
      expect(mapped.countFor(LibraryOrganizationQueue.pending), 4);
      expect(mapped.countFor(LibraryOrganizationQueue.deferred), 2);
      expect(mapped.countFor(LibraryOrganizationQueue.ignored), 1);
    },
  );
}
