import "dart:typed_data";

import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "assembles catalog chunks into one atomically returned manifest",
    () async {
      final revision = BigInt.from(7);
      const queryId = "query";
      final catalog = _FakeManifestCatalog([
        _chunk(
          revision: revision,
          queryId: queryId,
          totalItems: 3,
          startOrdinal: 0,
          locationIds: const ["one", "two"],
          nextCursor: _cursor(
            revision: revision,
            queryId: queryId,
            totalItems: 3,
            nextOrdinal: 2,
          ),
        ),
        _chunk(
          revision: revision,
          queryId: queryId,
          totalItems: 3,
          startOrdinal: 2,
          locationIds: const ["three"],
        ),
      ]);
      final loader = CatalogLibraryGalleryLayoutManifestLoader(catalog);

      final manifest = await loader.load(const LibraryGalleryQuery());

      expect(manifest.itemCount, 3);
      expect(manifest.locationIdAt(0), "one");
      expect(manifest.locationIdAt(2), "three");
      expect(catalog.requestedOrdinals, [null, 2]);
    },
  );

  test(
    "does not publish a partial manifest when a later chunk fails",
    () async {
      final revision = BigInt.from(7);
      const queryId = "query";
      final catalog = _FakeManifestCatalog([
        _chunk(
          revision: revision,
          queryId: queryId,
          totalItems: 3,
          startOrdinal: 0,
          locationIds: const ["one", "two"],
          nextCursor: _cursor(
            revision: revision,
            queryId: queryId,
            totalItems: 3,
            nextOrdinal: 2,
          ),
        ),
      ], failureAfterChunks: 1);
      final loader = CatalogLibraryGalleryLayoutManifestLoader(catalog);

      await expectLater(
        loader.load(const LibraryGalleryQuery()),
        throwsA(
          isA<LibraryGalleryLayoutManifestFailure>().having(
            (failure) => failure.code,
            "code",
            "catalog_changed",
          ),
        ),
      );
    },
  );

  test("stops requesting chunks after cancellation", () async {
    final revision = BigInt.from(7);
    const queryId = "query";
    final catalog = _FakeManifestCatalog([
      _chunk(
        revision: revision,
        queryId: queryId,
        totalItems: 3,
        startOrdinal: 0,
        locationIds: const ["one", "two"],
        nextCursor: _cursor(
          revision: revision,
          queryId: queryId,
          totalItems: 3,
          nextOrdinal: 2,
        ),
      ),
      _chunk(
        revision: revision,
        queryId: queryId,
        totalItems: 3,
        startOrdinal: 2,
        locationIds: const ["three"],
      ),
    ]);
    final loader = CatalogLibraryGalleryLayoutManifestLoader(catalog);

    await expectLater(
      loader.load(
        const LibraryGalleryQuery(),
        isCancelled: () => catalog.requestedOrdinals.isNotEmpty,
      ),
      throwsA(
        isA<LibraryGalleryLayoutManifestFailure>().having(
          (failure) => failure.code,
          "code",
          "gallery_layout_manifest_cancelled",
        ),
      ),
    );
    expect(catalog.requestedOrdinals, [null]);
  });
}

class _FakeManifestCatalog implements LibraryGalleryLayoutManifestCatalog {
  _FakeManifestCatalog(this._chunks, {this.failureAfterChunks});

  final List<LibraryGalleryLayoutManifestChunk> _chunks;
  final int? failureAfterChunks;
  final List<int?> requestedOrdinals = [];

  @override
  Future<LibraryGalleryLayoutManifestChunk> loadChunk({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryGalleryLayoutManifestCursor? after,
  }) async {
    expect(maxItems, libraryGalleryLayoutManifestChunkSize);
    requestedOrdinals.add(after?.nextOrdinal);
    if (failureAfterChunks == requestedOrdinals.length - 1) {
      throw const LibraryGalleryLayoutManifestFailure(
        code: "catalog_changed",
        message: "The catalog changed while the manifest was loading",
      );
    }
    return _chunks[requestedOrdinals.length - 1];
  }
}

LibraryGalleryLayoutManifestChunk _chunk({
  required BigInt revision,
  required String queryId,
  required int totalItems,
  required int startOrdinal,
  required List<String> locationIds,
  LibraryGalleryLayoutManifestCursor? nextCursor,
}) {
  return LibraryGalleryLayoutManifestChunk(
    revision: revision,
    queryId: queryId,
    totalItems: totalItems,
    startOrdinal: startOrdinal,
    locationIds: locationIds,
    aspectRatioMilli: Uint16List.fromList(
      List.filled(locationIds.length, 1000),
    ),
    dateGroupIndices: Uint16List(locationIds.length),
    dateGroups: const ["2026-08-09"],
    flags: Uint8List.fromList(
      List.filled(locationIds.length, libraryGalleryLayoutDimensionsKnownFlag),
    ),
    nextCursor: nextCursor,
  );
}

LibraryGalleryLayoutManifestCursor _cursor({
  required BigInt revision,
  required String queryId,
  required int totalItems,
  required int nextOrdinal,
}) {
  return LibraryGalleryLayoutManifestCursor(
    revision: revision,
    queryId: queryId,
    totalItems: totalItems,
    nextOrdinal: nextOrdinal,
    after: LibraryCatalogCursor(
      revision: revision,
      queryId: queryId,
      primaryMissing: false,
      primaryText: "",
      primaryNumber: 0,
      rootId: "root",
      locationId: "location-$nextOrdinal",
    ),
  );
}
