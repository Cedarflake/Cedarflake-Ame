import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("groups capture, creation, and modification fallback dates", () {
    final assets = [
      _asset("capture", captureLocalTime: "2026-01-02T03:04:05"),
      _asset(
        "created",
        createdUnixMs: DateTime(2025, 2, 3).millisecondsSinceEpoch,
        modifiedUnixMs: DateTime(2026, 4, 5).millisecondsSinceEpoch,
      ),
      _asset(
        "modified",
        modifiedUnixMs: DateTime(2024, 3, 4).millisecondsSinceEpoch,
      ),
    ];

    final entries = LibraryGalleryLayoutEntry.build(
      assets: assets,
      availableWidth: 600,
      layoutShape: GalleryLayoutShape.equalHeight,
      thumbnailSize: GalleryThumbnailSize.medium,
      sortKey: LibraryGallerySortKey.captureTime,
    );

    expect(
      entries.map((entry) => entry.headerLabel).whereType<String>().toList(),
      ["2026年1月2日", "2025年2月3日", "2024年3月4日"],
    );
    expect(
      entries
          .expand((entry) => entry.cells)
          .map((cell) => cell.asset.locationId)
          .toList(),
      ["capture", "created", "modified"],
    );
    expect(entries.every((entry) => entry.extent > 0), isTrue);
    final photoRows = entries.where((entry) => entry.cells.isNotEmpty);
    expect(
      photoRows.every(
        (entry) => entry.rowHeight == GalleryThumbnailSize.medium.targetExtent,
      ),
      isTrue,
    );

    final metrics = LibraryGalleryLayoutMetrics.fromEntries(
      entries,
      topPadding: 18,
      bottomPadding: 72,
    );
    expect(metrics.dateAnchors.map((anchor) => anchor.id), [
      "2026-01-02",
      "2025-02-03",
      "2024-03-04",
    ]);
    expect(metrics.offsetForLocation("capture"), isNotNull);
    expect(metrics.offsetForItemIndex(0), metrics.offsetForLocation("capture"));
    expect(metrics.itemIndexForScrollOffset(metrics.itemOffsets[1]), 1);
    expect(
      metrics.offsetForItemIndex(2),
      metrics.offsetForLocation("modified"),
    );
    expect(metrics.photoRowHeight, GalleryThumbnailSize.medium.targetExtent);
  });

  test("file name layout omits chronological headers", () {
    final entries = LibraryGalleryLayoutEntry.build(
      assets: [_asset("one"), _asset("two")],
      availableWidth: 500,
      layoutShape: GalleryLayoutShape.square,
      thumbnailSize: GalleryThumbnailSize.small,
      sortKey: LibraryGallerySortKey.fileName,
    );

    expect(entries.where((entry) => entry.headerLabel != null), isEmpty);
    expect(
      entries
          .expand((entry) => entry.cells)
          .map((cell) => cell.asset.locationId)
          .toList(),
      ["one", "two"],
    );
  });

  test("resolves a global item to the first item in its rendered row", () {
    final metrics = LibraryGalleryLayoutMetrics(
      contentExtent: 400,
      photoRowHeight: 100,
      dateAnchors: const [],
      locationOffsets: const {},
      itemOffsets: const [18, 18, 18, 124, 124],
      itemIndexBase: 100,
      isQueryWide: true,
    );

    expect(metrics.rowStartGlobalItemIndex(100), 100);
    expect(metrics.rowStartGlobalItemIndex(102), 100);
    expect(metrics.rowStartGlobalItemIndex(104), 103);
    expect(metrics.rowStartGlobalItemIndex(99), isNull);
    expect(metrics.rowStartGlobalItemIndex(105), isNull);
    expect(metrics.rowEndGlobalItemIndexExclusive(100), 103);
    expect(metrics.rowEndGlobalItemIndexExclusive(102), 103);
    expect(metrics.rowEndGlobalItemIndexExclusive(103), 105);
    expect(metrics.rowEndGlobalItemIndexExclusive(105), isNull);
  });
}

LibraryAsset _asset(
  String id, {
  String? captureLocalTime,
  int? createdUnixMs,
  int modifiedUnixMs = 1,
}) {
  return LibraryAsset(
    assetId: "asset-$id",
    locationId: id,
    rootId: "root",
    sourcePath: "C:\\Pictures\\$id.png",
    displayPath: "C:\\Pictures\\$id.png",
    relativePath: "$id.png",
    previewPath: "C:\\Cache\\$id.png",
    fileSize: BigInt.one,
    createdUnixMs: createdUnixMs,
    modifiedUnixMs: modifiedUnixMs,
    width: 160,
    height: 90,
    captureTime: captureLocalTime == null
        ? null
        : LibraryCaptureTimeEvidence(
            localTime: captureLocalTime,
            source: LibraryCaptureTimeSource.exifDateTimeOriginal,
            rawValue: captureLocalTime,
          ),
  );
}
