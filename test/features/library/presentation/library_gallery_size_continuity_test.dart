import "dart:typed_data";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_header.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final shape in GalleryLayoutShape.values) {
    for (final queryWide in [false, true]) {
      testWidgets(
        "$shape size round trips retain position, manifest=$queryWide",
        (tester) async {
          final scroll = await _openGallery(
            tester,
            shape: shape,
            queryWide: queryWide,
          );
          final viewport = find.descendant(
            of: find.byKey(const Key("library-photo-wall")),
            matching: find.byType(Scrollable),
          );
          for (final fraction in [0.31, 0.47, 0.61]) {
            scroll.jumpTo(scroll.maxScrollExtent * fraction);
            await tester.pumpAndSettle();
            final viewportRect = tester.getRect(viewport);
            final firstRow = find
                .byType(LibraryPhotoTile)
                .evaluate()
                .map((element) => tester.getRect(find.byWidget(element.widget)))
                .firstWhere((rect) => rect.overlaps(viewportRect));
            scroll.jumpTo(
              scroll.pixels +
                  firstRow.top -
                  viewportRect.top +
                  firstRow.height * 0.37,
            );
            await tester.pumpAndSettle();
            final pixels = scroll.pixels;
            final visible = <String, Rect>{};
            for (final element in find.byType(LibraryPhotoTile).evaluate()) {
              final tile = element.widget as LibraryPhotoTile;
              final rect = tester.getRect(find.byWidget(tile));
              if (rect.overlaps(viewportRect)) {
                visible[tile.asset.locationId] = rect;
              }
            }
            expect(visible, isNotEmpty);
            expect(
              visible.values.any((rect) => rect.top < viewportRect.top),
              isTrue,
              reason: "The fixture must retain a partially visible first row",
            );

            for (final size in [
              GalleryThumbnailSize.small,
              GalleryThumbnailSize.large,
              GalleryThumbnailSize.medium,
            ]) {
              tester
                  .widget<LibraryGalleryHeader>(
                    find.byType(LibraryGalleryHeader),
                  )
                  .onThumbnailSizeChanged(size);
              await tester.pumpAndSettle();
            }
            expect(
              scroll.pixels,
              closeTo(pixels, 0.01),
              reason: "Offset $fraction",
            );
            for (final entry in visible.entries) {
              final tile = find.byKey(ValueKey(entry.key));
              expect(tile, findsOneWidget);
              final rect = tester.getRect(tile);
              expect(rect.top, closeTo(entry.value.top, 0.01));
              expect(rect.left, closeTo(entry.value.left, 0.01));
              expect(rect.size, entry.value.size);
            }
          }
        },
      );
      for (final atEnd in [false, true]) {
        testWidgets("$shape retains edge, end=$atEnd, manifest=$queryWide", (
          tester,
        ) async {
          final scroll = await _openGallery(
            tester,
            shape: shape,
            queryWide: queryWide,
          );
          if (atEnd) {
            scroll.jumpTo(scroll.maxScrollExtent);
            await tester.pumpAndSettle();
          }
          for (final size in [
            GalleryThumbnailSize.large,
            GalleryThumbnailSize.small,
            GalleryThumbnailSize.medium,
          ]) {
            tester
                .widget<LibraryGalleryHeader>(find.byType(LibraryGalleryHeader))
                .onThumbnailSizeChanged(size);
            await tester.pumpAndSettle();
            expect(
              scroll.pixels,
              closeTo(atEnd ? scroll.maxScrollExtent : 0, 0.01),
              reason: "$size at edge",
            );
          }
        });
      }
      for (final itemCount in [2, 10]) {
        testWidgets(
          "$shape short gallery, count=$itemCount, manifest=$queryWide",
          (tester) async {
            final scroll = await _openGallery(
              tester,
              shape: shape,
              queryWide: queryWide,
              itemCount: itemCount,
              thumbnailSize: GalleryThumbnailSize.small,
            );
            expect(scroll.maxScrollExtent, 0);
            for (final size in [
              GalleryThumbnailSize.large,
              GalleryThumbnailSize.small,
              GalleryThumbnailSize.medium,
            ]) {
              tester
                  .widget<LibraryGalleryHeader>(
                    find.byType(LibraryGalleryHeader),
                  )
                  .onThumbnailSizeChanged(size);
              await tester.pumpAndSettle();
              expect(
                scroll.pixels,
                0,
                reason: "A gallery that fits initially remains at the start",
              );
              expect(find.byKey(const ValueKey("location-0")), findsOneWidget);
            }
            if (itemCount == 10) {
              tester
                  .widget<LibraryGalleryHeader>(
                    find.byType(LibraryGalleryHeader),
                  )
                  .onThumbnailSizeChanged(GalleryThumbnailSize.large);
              await tester.pumpAndSettle();
              expect(scroll.maxScrollExtent, greaterThan(0));
              scroll.jumpTo(scroll.maxScrollExtent);
              await tester.pumpAndSettle();
              for (final size in [
                GalleryThumbnailSize.small,
                GalleryThumbnailSize.large,
              ]) {
                tester
                    .widget<LibraryGalleryHeader>(
                      find.byType(LibraryGalleryHeader),
                    )
                    .onThumbnailSizeChanged(size);
                await tester.pumpAndSettle();
                expect(
                  scroll.pixels,
                  closeTo(scroll.maxScrollExtent, 0.01),
                  reason:
                      "End intent survives a temporarily non-scrollable layout",
                );
                if (size == GalleryThumbnailSize.small) {
                  expect(scroll.maxScrollExtent, 0);
                }
              }
            }
          },
        );
      }
    }
  }
}

Future<ScrollPosition> _openGallery(
  WidgetTester tester, {
  required GalleryLayoutShape shape,
  required bool queryWide,
  int itemCount = 180,
  GalleryThumbnailSize thumbnailSize = GalleryThumbnailSize.medium,
}) async {
  tester.view.physicalSize = const Size(1280, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final snapshot = _snapshot(itemCount: itemCount);
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(snapshot),
        ),
        initialLibraryViewPreferencesProvider.overrideWithValue(
          LibraryViewPreferences(
            layoutShape: shape,
            thumbnailSize: thumbnailSize,
          ),
        ),
        if (queryWide)
          libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(
            _ManifestLoader(snapshot),
          ),
      ],
      child: const AmeApp(),
    ),
  );
  await tester.pumpAndSettle();
  return tester
      .state<ScrollableState>(
        find.descendant(
          of: find.byKey(const Key("library-photo-wall")),
          matching: find.byType(Scrollable),
        ),
      )
      .position;
}

LibrarySnapshot _snapshot({int itemCount = 180}) => LibrarySnapshot(
  catalogPath: "catalog.sqlite3",
  revision: BigInt.one,
  queryId: "size-continuity",
  roots: [
    LibraryRoot(
      id: "root",
      path: "C:\\Pictures",
      displayPath: "C:\\Pictures",
      createdUnixMs: 1,
      assetCount: itemCount,
      issueCount: 0,
    ),
  ],
  assets: [
    for (var index = 0; index < itemCount; index++)
      LibraryAsset(
        assetId: "asset-$index",
        locationId: "location-$index",
        rootId: "root",
        activeScanId: "scan",
        sourceRevision: null,
        sourceGeneration: BigInt.one,
        sourcePath: "C:\\Pictures\\$index.jpg",
        displayPath: "C:\\Pictures\\$index.jpg",
        relativePath: "$index.jpg",
        previewPath: "C:\\Missing\\$index.jpg",
        fileSize: BigInt.one,
        modifiedUnixMs: 1,
        width: 4,
        height: 3,
        captureTime: const LibraryCaptureTimeEvidence(
          localTime: "2020-09-02T00:00:00.000000000",
          source: LibraryCaptureTimeSource.exifDateTimeOriginal,
          rawValue: "2020-09-02T00:00:00.000000000",
        ),
      ),
  ],
);

class _ManifestLoader implements LibraryGalleryLayoutManifestLoader {
  const _ManifestLoader(this.snapshot);

  final LibrarySnapshot snapshot;

  @override
  Future<LibraryGalleryLayoutManifest> load(
    LibraryGalleryQuery query, {
    bool Function()? isCancelled,
  }) async {
    final count = snapshot.assets.length;
    return (LibraryGalleryLayoutManifestBuilder(
          revision: snapshot.revision,
          queryId: snapshot.queryId,
          totalItems: count,
        )..append(
          LibraryGalleryLayoutManifestChunk(
            revision: snapshot.revision,
            queryId: snapshot.queryId,
            totalItems: count,
            startOrdinal: 0,
            locationIds: snapshot.assets
                .map((asset) => asset.locationId)
                .toList(),
            aspectRatioMilli: Uint16List.fromList(List.filled(count, 1333)),
            dateGroupIndices: Uint16List(count),
            dateGroups: const ["2020-09-02"],
            flags: Uint8List.fromList(
              List.filled(count, libraryGalleryLayoutDimensionsKnownFlag),
            ),
          ),
        ))
        .build();
  }
}
