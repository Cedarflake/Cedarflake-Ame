import "dart:typed_data";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";

LibraryState galleryState(int itemCount) {
  final revision = BigInt.one;
  final cursor = LibraryCatalogCursor(
    revision: revision,
    queryId: "resize-query",
    primaryMissing: false,
    primaryText: "2026-08-10T00:00:00.000000000",
    primaryNumber: 0,
    rootId: "root-1",
    locationId: "location-0",
  );
  const windowStart = 2800;
  final windowEnd = itemCount < 3201 ? itemCount : 3201;
  return LibraryState(
    status: LibraryStatus.completed,
    roots: [
      LibraryRoot(
        id: "root-1",
        path: "C:\\Pictures",
        displayPath: "C:\\Pictures",
        createdUnixMs: 1,
        assetCount: itemCount,
        issueCount: 0,
      ),
    ],
    catalogRevision: revision,
    queryId: "resize-query",
    windowStartItemOffset: windowStart,
    assets: [
      for (var index = windowStart; index < windowEnd; index++)
        galleryAsset(index, "initial preview evidence"),
    ],
    previousCursor: cursor,
    nextCursor: cursor,
    timeline: LibraryTimeline(
      revision: revision,
      queryId: "resize-query",
      totalItems: itemCount,
      buckets: [
        LibraryTimeBucket(
          monthKey: "2026-08",
          itemCount: itemCount,
          aspectRatioSum: itemCount.toDouble(),
        ),
      ],
    ),
  );
}

LibraryState galleryBoundaryState({
  required int itemCount,
  required int windowStart,
  required int windowEnd,
  required bool hasPrevious,
  required bool hasNext,
}) {
  final revision = BigInt.one;
  final cursor = LibraryCatalogCursor(
    revision: revision,
    queryId: "resize-query",
    primaryMissing: false,
    primaryText: "2026-08-10T00:00:00.000000000",
    primaryNumber: 0,
    rootId: "root-1",
    locationId: "location-$windowStart",
  );
  return LibraryState(
    status: LibraryStatus.completed,
    roots: [
      LibraryRoot(
        id: "root-1",
        path: "C:\\Pictures",
        displayPath: "C:\\Pictures",
        createdUnixMs: 1,
        assetCount: itemCount,
        issueCount: 0,
      ),
    ],
    catalogRevision: revision,
    queryId: "resize-query",
    windowStartItemOffset: windowStart,
    assets: [
      for (var index = windowStart; index < windowEnd; index++)
        galleryAsset(index, "boundary preview evidence"),
    ],
    previousCursor: hasPrevious ? cursor : null,
    nextCursor: hasNext ? cursor : null,
    timeline: LibraryTimeline(
      revision: revision,
      queryId: "resize-query",
      totalItems: itemCount,
      buckets: [
        LibraryTimeBucket(
          monthKey: "2026-08",
          itemCount: itemCount,
          aspectRatioSum: itemCount.toDouble(),
        ),
      ],
    ),
  );
}

LibraryAsset galleryAsset(int index, String previewIssueMessage) {
  return LibraryAsset(
    assetId: "asset-$index",
    locationId: "location-$index",
    rootId: "root-1",
    activeScanId: "scan-1",
    sourceRevision: null,
    sourceGeneration: BigInt.one,
    sourcePath: "C:\\Pictures\\$index.jpg",
    displayPath: "C:\\Pictures\\$index.jpg",
    relativePath: "$index.jpg",
    previewPath: "",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 4,
    height: 3,
    previewStatus: LibraryPreviewStatus.failed,
    previewIssueCode: "preview_failed",
    previewIssueMessage: previewIssueMessage,
  );
}

LibraryGalleryLayoutManifest galleryManifest(
  int itemCount, {
  bool dimensionsKnown = true,
}) {
  final revision = BigInt.one;
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: revision,
    queryId: "resize-query",
    totalItems: itemCount,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: revision,
      queryId: "resize-query",
      totalItems: itemCount,
      startOrdinal: 0,
      locationIds: [
        for (var index = 0; index < itemCount; index++) "location-$index",
      ],
      aspectRatioMilli: Uint16List.fromList([
        for (var index = 0; index < itemCount; index++)
          dimensionsKnown ? 650 + (index * 277) % 2350 : 1000,
      ]),
      dateGroupIndices: Uint16List.fromList([
        for (var index = 0; index < itemCount; index++) index ~/ 500,
      ]),
      dateGroups: const [
        "2026-08-10",
        "2026-08-09",
        "2026-08-08",
        "2026-08-07",
        "2026-08-06",
        "2026-08-05",
        "2026-08-04",
        "2026-08-03",
      ],
      flags: Uint8List.fromList(
        List.filled(
          itemCount,
          dimensionsKnown ? libraryGalleryLayoutDimensionsKnownFlag : 0,
        ),
      ),
    ),
  );
  return builder.build();
}

class RecordingGalleryController extends LibraryController {
  var nextPageRequests = 0;
  var previousPageRequests = 0;
  var timeSeekRequests = 0;
  final visibleRangeRequests = <({int start, int end})>[];
  final previewDemandRequests =
      <
        ({
          List<String> visible,
          List<String> nearDirection,
          List<String> guard,
          Map<String, int> edges,
        })
      >[];

  @override
  void updateGalleryPreviewDemand({
    Iterable<LibraryAsset> visible = const <LibraryAsset>[],
    Iterable<LibraryAsset> nearDirection = const <LibraryAsset>[],
    Iterable<LibraryAsset> guard = const <LibraryAsset>[],
    Iterable<LibraryAsset> idle = const <LibraryAsset>[],
    Map<String, int> previewEdges = const <String, int>{},
  }) {
    previewDemandRequests.add((
      visible: [for (final asset in visible) asset.locationId],
      nearDirection: [for (final asset in nearDirection) asset.locationId],
      guard: [for (final asset in guard) asset.locationId],
      edges: Map.of(previewEdges),
    ));
  }

  @override
  void ensureVisibleRange({
    required int startItemOffset,
    required int endItemOffsetExclusive,
  }) {
    visibleRangeRequests.add((
      start: startItemOffset,
      end: endItemOffsetExclusive,
    ));
  }

  @override
  Future<void> loadNextPage() async {
    nextPageRequests += 1;
  }

  @override
  Future<bool> loadPreviousPage() async {
    previousPageRequests += 1;
    return false;
  }

  @override
  Future<bool> jumpToTime(
    LibraryTimeBucket bucket, {
    int itemOffset = 0,
  }) async {
    timeSeekRequests += 1;
    return false;
  }
}
