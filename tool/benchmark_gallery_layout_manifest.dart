import "dart:convert";
import "dart:io";
import "dart:typed_data";

import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";

const _chunkSize = 4096;
const _itemCounts = [79013, 250000, 1000000];

void main() {
  final results = <Map<String, Object>>[];
  for (final itemCount in _itemCounts) {
    results.add(_benchmark(itemCount));
  }
  stdout.writeln(
    const JsonEncoder.withIndent("  ").convert({"results": results}),
  );
}

Map<String, Object> _benchmark(int itemCount) {
  final plan = LibraryGalleryLayoutManifestStoragePlan.forItemCount(itemCount);
  final initialRss = ProcessInfo.currentRss;
  final stopwatch = Stopwatch()..start();
  final revision = BigInt.one;
  const queryId = "synthetic-layout-manifest";
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: revision,
    queryId: queryId,
    totalItems: itemCount,
  );
  for (var start = 0; start < itemCount; start += _chunkSize) {
    final count = (itemCount - start).clamp(0, _chunkSize).toInt();
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
  stopwatch.stop();

  return {
    "items": itemCount,
    "storage": plan.kind.name,
    "estimated_bytes": plan.estimatedBytes,
    "built": true,
    "build_milliseconds": stopwatch.elapsedMilliseconds,
    "primitive_bytes": manifest.primitiveByteLength,
    "primitive_bytes_per_item": manifest.primitiveByteLength / itemCount,
    "retained_rss_delta_bytes": ProcessInfo.currentRss - initialRss,
  };
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
      rootId: "synthetic-root",
      locationId: "synthetic-$nextOrdinal",
    ),
  );
}
