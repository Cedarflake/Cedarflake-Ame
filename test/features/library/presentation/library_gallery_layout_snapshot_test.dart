import "dart:typed_data";

import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout_snapshot.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("builds one query-wide equal-height wall from manifest geometry", () {
    final manifest = _manifest();

    final snapshot = LibraryGalleryLayoutSnapshot.build(
      manifest: manifest,
      availableWidth: 300,
      thumbnailSize: GalleryThumbnailSize.small,
      sortKey: LibraryGallerySortKey.captureTime,
    );

    final rows = snapshot.entries.where((entry) => entry.isPhotoRow).toList();
    expect(rows, hasLength(2));
    expect(rows[0].startItemIndex, 0);
    expect(rows[0].itemCount, 3);
    expect(rows[0].cellWidths, everyElement(96));
    expect(rows[1].startItemIndex, 3);
    expect(rows[1].itemCount, 3);
    expect(snapshot.metrics.isQueryWide, isTrue);
    expect(snapshot.metrics.itemOffsets, hasLength(6));
    expect(snapshot.metrics.itemOffsets[0], snapshot.metrics.itemOffsets[2]);
    expect(snapshot.metrics.itemOffsets[3], snapshot.metrics.itemOffsets[5]);
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
    expect(snapshot.entries.where((entry) => entry.isPhotoRow), hasLength(2));
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
