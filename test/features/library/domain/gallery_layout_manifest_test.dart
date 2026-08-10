import "dart:typed_data";

import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("assembles the accepted real-library scale into compact columns", () {
    const itemCount = 79013;
    const chunkSize = 4096;
    final revision = BigInt.from(13);
    const queryId = "layout-query";
    final builder = LibraryGalleryLayoutManifestBuilder(
      revision: revision,
      queryId: queryId,
      totalItems: itemCount,
    );

    for (var start = 0; start < itemCount; start += chunkSize) {
      final count = (itemCount - start).clamp(0, chunkSize).toInt();
      final end = start + count;
      builder.append(
        LibraryGalleryLayoutManifestChunk(
          revision: revision,
          queryId: queryId,
          totalItems: itemCount,
          startOrdinal: start,
          locationIds: [
            for (var index = start; index < end; index++)
              index.toRadixString(16).padLeft(64, "0"),
          ],
          aspectRatioMilli: Uint16List.fromList([
            for (var index = start; index < end; index++) 200 + (index % 4801),
          ]),
          dateGroupIndices: Uint16List.fromList([
            for (var index = start; index < end; index++) index.isEven ? 0 : 1,
          ]),
          dateGroups: const ["2026-08-09", "2026-08-08"],
          flags: Uint8List.fromList([
            for (var index = start; index < end; index++)
              index % 17 == 0 ? 0 : libraryGalleryLayoutDimensionsKnownFlag,
          ]),
          nextCursor: end == itemCount
              ? null
              : _cursor(
                  revision: revision,
                  queryId: queryId,
                  totalItems: itemCount,
                  nextOrdinal: end,
                ),
        ),
      );
    }

    final manifest = builder.build();

    expect(manifest.itemCount, itemCount);
    expect(manifest.locationIdAt(0), "0".padLeft(64, "0"));
    expect(
      manifest.locationIdAt(itemCount - 1),
      (itemCount - 1).toRadixString(16).padLeft(64, "0"),
    );
    expect(manifest.aspectRatioAt(4800), 5);
    expect(manifest.dateKeyAt(1), "2026-08-08");
    expect(manifest.hasKnownDimensionsAt(0), isFalse);
    expect(manifest.hasKnownDimensionsAt(1), isTrue);
    expect(manifest.primitiveByteLength, lessThan(6 * 1024 * 1024));
  });

  test(
    "selects hierarchical storage before a flat manifest exceeds budget",
    () {
      expect(
        LibraryGalleryLayoutManifestStoragePlan.forItemCount(79013).kind,
        LibraryGalleryLayoutManifestStorageKind.flat,
      );
      expect(
        LibraryGalleryLayoutManifestStoragePlan.forItemCount(250000).kind,
        LibraryGalleryLayoutManifestStorageKind.flat,
      );
      expect(
        LibraryGalleryLayoutManifestStoragePlan.forItemCount(1000000).kind,
        LibraryGalleryLayoutManifestStorageKind.hierarchical,
      );
      final builder = LibraryGalleryLayoutManifestBuilder(
        revision: BigInt.one,
        queryId: "over-budget",
        totalItems: 1000000,
      );
      expect(
        builder.storageKind,
        LibraryGalleryLayoutManifestStorageKind.hierarchical,
      );
    },
  );

  test("rejects discontinuous chunks and mismatched columns", () {
    final revision = BigInt.one;
    final builder = LibraryGalleryLayoutManifestBuilder(
      revision: revision,
      queryId: "query",
      totalItems: 2,
    );
    final discontinuous = LibraryGalleryLayoutManifestChunk(
      revision: revision,
      queryId: "query",
      totalItems: 2,
      startOrdinal: 1,
      locationIds: const ["one"],
      aspectRatioMilli: Uint16List.fromList([1000]),
      dateGroupIndices: Uint16List.fromList([0]),
      dateGroups: const ["2026-08-09"],
      flags: Uint8List.fromList([1]),
      nextCursor: _cursor(
        revision: revision,
        queryId: "query",
        totalItems: 2,
        nextOrdinal: 2,
      ),
    );

    expect(
      () => builder.append(discontinuous),
      throwsA(
        isA<LibraryGalleryLayoutManifestFailure>().having(
          (failure) => failure.code,
          "code",
          "gallery_layout_manifest_chunk_mismatch",
        ),
      ),
    );

    final invalidColumns = LibraryGalleryLayoutManifestChunk(
      revision: revision,
      queryId: "query",
      totalItems: 2,
      startOrdinal: 0,
      locationIds: const ["one"],
      aspectRatioMilli: Uint16List(0),
      dateGroupIndices: Uint16List.fromList([0]),
      dateGroups: const ["2026-08-09"],
      flags: Uint8List.fromList([1]),
      nextCursor: _cursor(
        revision: revision,
        queryId: "query",
        totalItems: 2,
        nextOrdinal: 1,
      ),
    );

    expect(
      () => builder.append(invalidColumns),
      throwsA(
        isA<LibraryGalleryLayoutManifestFailure>().having(
          (failure) => failure.code,
          "code",
          "gallery_layout_manifest_columns_invalid",
        ),
      ),
    );
  });
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
