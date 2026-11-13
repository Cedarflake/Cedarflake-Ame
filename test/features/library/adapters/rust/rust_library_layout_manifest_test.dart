import "dart:typed_data";

import "package:cedarflake_ame/features/library/adapters/rust/rust_library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/src/rust/domain.dart" as rust;
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "snapshot conversion yields to input before publishing the complete layout",
    () async {
      final events = <String>[];
      final input = Future<void>(() => events.add("input"));
      final manifest = await mapRustGalleryLayoutManifest([
        _chunk(0),
        _chunk(1),
      ]);
      events.add("published");
      await input;

      expect(events, ["input", "published"]);
      expect(manifest!.itemCount, 2);
      expect(manifest.locationIdAt(0), "photo-0");
      expect(manifest.locationIdAt(1), "photo-1");
      expect(manifest.entryIdAt(1), "group-1");
      expect(manifest.aspectRatioAt(1), 1.5);
      expect(manifest.dateKeyAt(1), "2026-11-13");
    },
  );

  test(
    "a later inconsistent chunk still rejects the entire snapshot",
    () async {
      await expectLater(
        mapRustGalleryLayoutManifest([
          _chunk(0),
          _chunk(1, revision: BigInt.two),
        ]),
        throwsA(
          isA<LibraryGalleryLayoutManifestFailure>().having(
            (failure) => failure.code,
            "code",
            "gallery_layout_manifest_chunk_mismatch",
          ),
        ),
      );
    },
  );
}

rust.GalleryLayoutManifestChunk _chunk(int index, {BigInt? revision}) {
  return rust.GalleryLayoutManifestChunk(
    revision: revision ?? BigInt.one,
    queryId: "gallery",
    totalItems: BigInt.two,
    startOrdinal: BigInt.from(index),
    locationIds: ["photo-$index"],
    entryIds: ["group-$index"],
    aspectRatioMilli: Uint16List.fromList([1500]),
    dateGroupIndices: Uint16List(1),
    dateGroups: const [rust.GalleryLayoutDateGroup(dateKey: "2026-11-13")],
    flags: Uint8List.fromList([1]),
    nextCursor: index == 0
        ? rust.GalleryLayoutManifestCursor(
            revision: BigInt.one,
            queryId: "gallery",
            totalItems: BigInt.two,
            nextOrdinal: BigInt.one,
            after: rust.CatalogCursor(
              revision: BigInt.one,
              queryId: "gallery",
              primaryMissing: false,
              primaryText: "2026-11-13",
              primaryNumber: 0,
              rootId: "root",
              locationId: "photo-0",
            ),
          )
        : null,
  );
}
