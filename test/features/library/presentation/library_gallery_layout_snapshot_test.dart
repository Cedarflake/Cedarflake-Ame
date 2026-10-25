import "dart:typed_data";

import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/justified_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_grouping.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout_snapshot.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_reflow.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final queue in LibraryOrganizationQueue.values) {
    for (final sortKey in LibraryGallerySortKey.values) {
      for (final shape in GalleryLayoutShape.values) {
        test("$queue uses sort-specific $shape rows for $sortKey", () {
          final query = LibraryGalleryQuery(
            organizationQueue: queue,
            sortKey: sortKey,
          );
          final grouping = LibraryGalleryGrouping.forQuery(query);
          final manifest = _manifest(
            locationIds: ["first", "second", "third"],
            aspectRatios: [1, 1, 1],
            dateKeys: [null, "2026-10-24", "2025-01-01"],
          );
          final snapshot = LibraryGalleryLayoutSnapshot.build(
            manifest: manifest,
            availableWidth: 900,
            thumbnailSize: GalleryThumbnailSize.small,
            layoutShape: shape,
            sortKey: sortKey,
            grouping: grouping,
          );
          expect(
            snapshot.metrics.dateAnchors.map((anchor) => anchor.year),
            query.isChronological ? [null, 2026, 2025] : isEmpty,
          );
          expect(
            snapshot.entries
                .where((entry) => entry.isPhotoRow)
                .map((entry) => entry.itemCount),
            query.isChronological ? [1, 1, 1] : [3],
          );
          expect(
            snapshot.entries.where((entry) => entry.headerLabel != null),
            hasLength(query.isChronological ? 3 : 0),
          );
          expect(
            snapshot.matchesInputs(
              otherManifest: manifest,
              otherThumbnailSize: GalleryThumbnailSize.small,
              otherLayoutShape: shape,
              otherSortKey: sortKey,
            ),
            isTrue,
          );
          final assets = [
            for (var index = 0; index < 3; index++)
              LibraryAsset(
                assetId: "asset-$index",
                locationId: "location-$index",
                rootId: "root",
                activeScanId: "scan",
                sourcePath: "C:\\Fixtures\\$index.png",
                displayPath: "C:\\Fixtures\\$index.png",
                relativePath: "$index.png",
                previewPath: "C:\\Missing\\$index.jpg",
                fileSize: BigInt.one,
                createdUnixMs: index * 86400000,
                modifiedUnixMs: index * 86400000,
                sourceRevision: null,
                sourceGeneration: BigInt.one,
                width: 100,
                height: 100,
              ),
          ];
          final loaded = LibraryGalleryLayoutEntry.build(
            assets: assets,
            availableWidth: 900,
            layoutShape: shape,
            thumbnailSize: GalleryThumbnailSize.small,
            sortKey: sortKey,
            grouping: grouping,
          );
          final loadedRows = loaded.where((entry) => entry.cells.isNotEmpty);
          expect(loadedRows, hasLength(query.isChronological ? 3 : 1));
          expect(
            loaded.where((entry) => entry.headerLabel != null),
            hasLength(query.isChronological ? 3 : 0),
          );
          expect(
            loadedRows
                .expand((row) => row.cells)
                .map((cell) => cell.asset.locationId),
            assets.map((asset) => asset.locationId),
          );
        });
      }
    }
  }

  test("query-wide and loaded rows share packing across all densities", () {
    final ratios = [
      for (var i = 0; i < 6; i++) 1.0,
      for (var i = 0; i < 6; i++) 0.125,
      8.0,
      1.0,
    ];
    final manifest = _manifest(
      locationIds: List.generate(ratios.length, (index) => "image-$index"),
      aspectRatios: ratios,
      dateKeys: List.filled(ratios.length, "2026-08-09"),
    );
    for (final size in GalleryThumbnailSize.values) {
      final snapshot = LibraryGalleryLayoutSnapshot.build(
        manifest: manifest,
        availableWidth: 816,
        thumbnailSize: size,
        sortKey: LibraryGallerySortKey.captureTime,
      );
      final rows = snapshot.entries.where((entry) => entry.isPhotoRow).toList();
      final loaded = JustifiedGalleryLayout(
        targetRowHeight: size.targetExtent,
        spacing: 6,
      ).compute(aspectRatios: ratios, availableWidth: 816);
      expect(rows.length, loaded.length);
      for (var i = 0; i < rows.length; i++) {
        expect(rows[i].startItemIndex, loaded[i].cells.first.itemIndex);
        expect(rows[i].cellWidths.length, loaded[i].cells.length);
        for (var j = 0; j < rows[i].cellWidths.length; j++) {
          expect(
            rows[i].cellWidths[j],
            closeTo(loaded[i].cells[j].width, 0.001),
          );
        }
      }
    }
  });

  for (final shape in GalleryLayoutShape.values) {
    test("$shape reserves scaled card width before detail loading", () {
      final manifest = _manifest(aspectRatios: List.filled(6, 0.01));
      final snapshot = LibraryGalleryLayoutSnapshot.build(
        manifest: manifest,
        availableWidth: 600,
        minimumCellWidth: 256,
        thumbnailSize: GalleryThumbnailSize.small,
        layoutShape: shape,
        sortKey: LibraryGallerySortKey.captureTime,
      );
      for (final row in snapshot.entries.where((entry) => entry.isPhotoRow)) {
        expect(row.cellWidths, everyElement(greaterThanOrEqualTo(256)));
      }
      expect(
        snapshot.matches(
          otherManifest: manifest,
          otherAvailableWidth: 600,
          otherMinimumCellWidth: 128,
          otherThumbnailSize: GalleryThumbnailSize.small,
          otherLayoutShape: shape,
          otherSortKey: LibraryGallerySortKey.captureTime,
        ),
        isFalse,
      );
      expect(
        snapshot.matches(
          otherManifest: manifest,
          otherAvailableWidth: 600,
          otherMinimumCellWidth: 256,
          otherThumbnailSize: GalleryThumbnailSize.small,
          otherLayoutShape: shape,
          otherSortKey: LibraryGallerySortKey.captureTime,
        ),
        isTrue,
      );
    });
  }

  test("builds one query-wide equal-height wall from manifest geometry", () {
    final manifest = _manifest();

    final snapshot = LibraryGalleryLayoutSnapshot.build(
      manifest: manifest,
      availableWidth: 300,
      thumbnailSize: GalleryThumbnailSize.small,
      sortKey: LibraryGallerySortKey.captureTime,
    );

    final rows = snapshot.entries.where((entry) => entry.isPhotoRow).toList();
    expect(rows, hasLength(4));
    expect(rows[0].startItemIndex, 0);
    expect(rows[0].itemCount, 2);
    expect(rows[0].cellWidths, everyElement(147));
    expect(rows[1].startItemIndex, 2);
    expect(rows[1].itemCount, 1);
    expect(rows[1].cellWidths, everyElement(128));
    expect(snapshot.metrics.isQueryWide, isTrue);
    expect(snapshot.metrics.itemOffsets, hasLength(6));
    expect(snapshot.metrics.itemOffsets[0], snapshot.metrics.itemOffsets[1]);
    expect(snapshot.metrics.itemOffsets[3], snapshot.metrics.itemOffsets[4]);
    expect(
      snapshot.metrics.itemOffsets[3],
      greaterThan(snapshot.metrics.itemOffsets[2]),
    );
  });

  test("keeps exact wall geometry while the loaded detail window changes", () {
    final snapshot = LibraryGalleryLayoutSnapshot.build(
      manifest: _manifest(),
      availableWidth: 300,
      thumbnailSize: GalleryThumbnailSize.small,
      sortKey: LibraryGallerySortKey.captureTime,
    );

    final firstWindow = snapshot.loadedWindowGeometry(
      startItemIndex: 0,
      itemCount: 3,
    );
    final secondWindow = snapshot.loadedWindowGeometry(
      startItemIndex: 3,
      itemCount: 3,
    );

    expect(firstWindow.leading, lessThan(secondWindow.leading));
    expect(
      firstWindow.leading + firstWindow.content + firstWindow.trailing,
      closeTo(snapshot.metrics.contentExtent, 0.01),
    );
    expect(
      secondWindow.leading + secondWindow.content + secondWindow.trailing,
      closeTo(snapshot.metrics.contentExtent, 0.01),
    );
    expect(snapshot.entries.where((entry) => entry.isPhotoRow), hasLength(4));
  });

  test("treats malformed date keys as an unknown group", () {
    final manifest = _manifest(
      locationIds: const ["one", "two"],
      aspectRatios: const [1, 1],
      dateKeys: const ["x", "2026-08-09"],
    );

    final snapshot = LibraryGalleryLayoutSnapshot.build(
      manifest: manifest,
      availableWidth: 600,
      thumbnailSize: GalleryThumbnailSize.medium,
      sortKey: LibraryGallerySortKey.captureTime,
    );

    expect(snapshot.metrics.dateAnchors.first.isUnknown, isTrue);
    expect(snapshot.metrics.dateAnchors.first.year, isNull);
  });

  for (final pointInHeading in [false, true]) {
    test(
      "manifest anchor round-trips a ${pointInHeading ? 'date heading' : 'row gap'} without displacement",
      () {
        final snapshot = LibraryGalleryLayoutSnapshot.build(
          manifest: _manifest(
            locationIds: List.generate(18, (index) => "location-$index"),
            aspectRatios: List.filled(18, 1),
            dateKeys: List.generate(
              18,
              (index) => index < 9 ? "2026-08-09" : "2012-03-04",
            ),
          ),
          availableWidth: 300,
          thumbnailSize: GalleryThumbnailSize.small,
          sortKey: LibraryGallerySortKey.captureTime,
        );
        final rows = snapshot.entries
            .where((entry) => entry.isPhotoRow)
            .toList();
        final nextRow = pointInHeading
            ? rows.firstWhere((row) => row.startItemIndex == 9)
            : rows[1];
        final rowOffset = snapshot.metrics.offsetForGlobalItemIndex(
          nextRow.startItemIndex,
        )!;
        const viewport = 100.0;
        final pixels = rowOffset - (pointInHeading ? 13 : 2) - viewport * 0.5;
        final anchor = LibraryGalleryViewportAnchor.capture(
          snapshot,
          scrollOffset: pixels,
          viewportExtent: viewport,
          viewportFraction: 0.5,
        )!;

        expect(anchor.itemFraction, inInclusiveRange(0.0, 1.0));
        expect(anchor.viewportFraction, isNot(0.5));
        expect(
          anchor.scrollOffsetFor(snapshot, viewport),
          closeTo(pixels, 1e-9),
        );
      },
    );
  }
}

LibraryGalleryLayoutManifest _manifest({
  List<String>? locationIds,
  List<double>? aspectRatios,
  List<String?>? dateKeys,
}) {
  final resolvedLocationIds =
      locationIds ?? const ["a", "b", "c", "d", "e", "f"];
  final resolvedAspectRatios = aspectRatios ?? const [1, 1, 1, 1, 1, 0];
  final resolvedDateKeys =
      dateKeys ??
      const [
        "2026-08-09",
        "2026-08-09",
        "2026-08-09",
        "2026-08-08",
        "2026-08-08",
        "2026-08-08",
      ];
  assert(resolvedLocationIds.length == resolvedAspectRatios.length);
  assert(resolvedLocationIds.length == resolvedDateKeys.length);
  final dateGroups = <String?>[];
  final dateGroupLookup = <String?, int>{};
  final dateGroupIndices = <int>[];
  for (final dateKey in resolvedDateKeys) {
    dateGroupIndices.add(
      dateGroupLookup.putIfAbsent(dateKey, () {
        dateGroups.add(dateKey);
        return dateGroups.length - 1;
      }),
    );
  }
  final revision = BigInt.from(7);
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: revision,
    queryId: "query-wide-layout",
    totalItems: resolvedLocationIds.length,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: revision,
      queryId: "query-wide-layout",
      totalItems: resolvedLocationIds.length,
      startOrdinal: 0,
      locationIds: resolvedLocationIds,
      aspectRatioMilli: Uint16List.fromList([
        for (final ratio in resolvedAspectRatios) (ratio * 1000).round(),
      ]),
      dateGroupIndices: Uint16List.fromList(dateGroupIndices),
      dateGroups: dateGroups,
      flags: Uint8List.fromList([
        for (final ratio in resolvedAspectRatios)
          if (ratio > 0) libraryGalleryLayoutDimensionsKnownFlag else 0,
      ]),
    ),
  );
  return builder.build();
}
