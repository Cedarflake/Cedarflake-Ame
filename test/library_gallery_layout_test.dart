import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("groups capture dates while preserving gallery order", () {
    final assets = [
      _asset("one", captureLocalTime: "2026-01-02T03:04:05"),
      _asset("two", captureLocalTime: "2026-01-02T10:11:12"),
      _asset("unknown"),
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
      ["2026年1月2日", LibraryStrings.unknownCaptureDate],
    );
    expect(
      entries
          .expand((entry) => entry.cells)
          .map((cell) => cell.asset.locationId)
          .toList(),
      ["one", "two", "unknown"],
    );
    expect(entries.every((entry) => entry.extent > 0), isTrue);
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
}

LibraryAsset _asset(String id, {String? captureLocalTime}) {
  return LibraryAsset(
    assetId: "asset-$id",
    locationId: id,
    rootId: "root",
    sourcePath: "C:\\Pictures\\$id.png",
    relativePath: "$id.png",
    previewPath: "C:\\Cache\\$id.png",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
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
