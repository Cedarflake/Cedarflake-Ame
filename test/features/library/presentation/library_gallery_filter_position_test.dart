import "dart:async";
import "dart:typed_data";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/application/ports/library_catalog.dart";
import "package:cedarflake_ame/features/library/domain/library_query_snapshot.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_filter.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_smart_album_navigation.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter/gestures.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_query_snapshot_fixture.dart";

import "../../../support/application_test_bindings.dart";

void main() {
  testWidgets("timeline input retires an earlier position refresh", (
    tester,
  ) async {
    final catalog = await _openGallery(
      tester,
      GalleryLayoutShape.equalHeight,
      useManifest: true,
    );
    final container = ProviderScope.containerOf(
      tester.element(find.byType(AmeApp)),
    );
    final read = Completer<void>();
    catalog.pendingRead = read.future;
    final refresh = container
        .read(libraryControllerProvider.notifier)
        .refreshCurrentQuery();
    await tester.pump();
    final track = tester.getRect(
      find.byKey(const Key("timeline-track-background")),
    );
    await tester.tapAt(
      Offset(track.center.dx, track.top + track.height * .75),
      kind: PointerDeviceKind.mouse,
    );
    await tester.pumpAndSettle();
    expect(
      _scroll(tester).pixels,
      greaterThan(_scroll(tester).maxScrollExtent * .6),
    );
    await tester.tapAt(
      Offset(track.center.dx, track.top + track.height * .4),
      kind: PointerDeviceKind.mouse,
    );
    await tester.pumpAndSettle();
    final targetPixels = _scroll(tester).pixels;
    expect(
      targetPixels,
      inExclusiveRange(
        _scroll(tester).maxScrollExtent * .25,
        _scroll(tester).maxScrollExtent * .55,
      ),
    );
    read.complete();
    await tester.pumpAndSettle();
    await refresh;
    expect(_scroll(tester).pixels, closeTo(targetPixels, .01));
  });

  testWidgets("switching smart albums resets a middle position", (
    tester,
  ) async {
    final catalog = await _openGallery(
      tester,
      GalleryLayoutShape.equalHeight,
      useManifest: true,
    );
    await tester.tap(find.byKey(const Key("workbench-albums")));
    await tester.pumpAndSettle();
    void selectCategory(String id) {
      tester
          .widget<LibrarySmartAlbumNavigation>(
            find.byType(LibrarySmartAlbumNavigation),
          )
          .onSelect(
            LibraryClassificationCategory(
              id: id,
              label: id,
              descriptions: const ["fixture"],
            ),
          );
    }

    selectCategory("drawing");
    await tester.pumpAndSettle();
    _scroll(tester).jumpTo(_scroll(tester).maxScrollExtent * .6);
    await tester.pumpAndSettle();
    expect(_scroll(tester).pixels, greaterThan(1000));
    final manifest = Completer<void>();
    catalog.pendingManifest = manifest.future;
    selectCategory("photo");
    await tester.pumpAndSettle();
    expect(_scroll(tester).pixels, 0);
    manifest.complete();
    await tester.pumpAndSettle();
    expect(_scroll(tester).pixels, 0);
    expect(find.byKey(const ValueKey("location-150")), findsOneWidget);
  });

  for (final shape in GalleryLayoutShape.values) {
    testWidgets("$shape opens a complete snapshot at the date heading", (
      tester,
    ) async {
      await _openGallery(
        tester,
        shape,
        useManifest: true,
        initialManifest: true,
      );
      expect(_scroll(tester).pixels, 0);
      final wall = tester.getRect(find.byKey(const Key("library-photo-wall")));
      final heading = tester.getRect(find.text("2020年9月2日").first);
      expect(heading.top, greaterThanOrEqualTo(wall.top));
      expect(heading.bottom, lessThan(wall.bottom));
    });
    for (final filter in ["one", "bottom", "excluded", "empty"]) {
      testWidgets("$shape restores middle pixels after filter $filter", (
        tester,
      ) async {
        final catalog = await _openGallery(tester, shape);
        _scroll(tester).jumpTo(_scroll(tester).maxScrollExtent * 0.43);
        await tester.pumpAndSettle();
        final originalPixels = _scroll(tester).pixels;
        await _filter(tester, filter, metadata: filter == "one");
        final original = catalog.requests.first!;
        if (filter == "one") {
          expect(_scroll(tester).pixels, 0);
          expect(_scroll(tester).maxScrollExtent, 0);
        } else if (filter == "bottom") {
          expect(_scroll(tester).maxScrollExtent, greaterThan(0));
          expect(
            _scroll(tester).pixels,
            closeTo(_scroll(tester).maxScrollExtent, 0.01),
          );
        } else if (filter == "excluded") {
          final wall = tester.getRect(
            find.descendant(
              of: find.byKey(const Key("library-photo-wall")),
              matching: find.byType(Scrollable),
            ),
          );
          expect(
            tester.getRect(find.byKey(const ValueKey("location-0"))).top,
            closeTo(wall.top, 0.01),
          );
        }
        await _filter(tester, "", metadata: filter == "one");
        expect(
          catalog.requests.last?.requestedLocationId,
          original.requestedLocationId,
        );
        expect(_scroll(tester).pixels, closeTo(originalPixels, 0.01));
      });
    }

    testWidgets("$shape adopts browsing after a filter fallback", (
      tester,
    ) async {
      final catalog = await _openGallery(tester, shape);
      _scroll(tester).jumpTo(_scroll(tester).maxScrollExtent * 0.43);
      await tester.pumpAndSettle();
      await _filter(tester, "excluded");
      final original = catalog.requests.first!;
      await tester.drag(
        find.byKey(const Key("library-photo-wall")),
        const Offset(0, -400),
      );
      await tester.pumpAndSettle();
      expect(_scroll(tester).pixels, greaterThan(0));
      await _filter(tester, "");
      expect(
        catalog.requests.last?.requestedLocationId,
        isNot(original.requestedLocationId),
      );
    });
  }
}

ScrollPosition _scroll(WidgetTester tester) => tester
    .state<ScrollableState>(
      find.descendant(
        of: find.byKey(const Key("library-photo-wall")),
        matching: find.byType(Scrollable),
      ),
    )
    .position;

Future<void> _filter(
  WidgetTester tester,
  String text, {
  bool metadata = false,
}) async {
  final filter = tester.widget<LibraryGalleryFilter>(
    find.byType(LibraryGalleryFilter),
  );
  filter.onChanged(
    metadata
        ? filter.query.copyWith(
            metadataFilter: LibraryMetadataFilter(extensions: [text]),
          )
        : filter.query.copyWith(searchText: text),
  );
  await tester.pumpAndSettle();
}

Future<_Catalog> _openGallery(
  WidgetTester tester,
  GalleryLayoutShape shape, {
  bool useManifest = false,
  bool initialManifest = false,
}) async {
  tester.view.physicalSize = const Size(1280, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final catalog = _Catalog();
  final initial = catalog.snapshot(const LibraryGalleryQuery());
  await tester.pumpWidget(
    ProviderScope(
      overrides: applicationTestOverrides([
        libraryCatalogProvider.overrideWithValue(catalog),
        if (useManifest)
          libraryGalleryLayoutManifestProvider.overrideWith((
            ref,
            request,
          ) async {
            await catalog.pendingManifest;
            return catalog.manifest(request.query);
          }),
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initial.snapshot).copyWith(
            timeline: initial.timeline,
            layoutManifest: initialManifest
                ? catalog.manifest(const LibraryGalleryQuery())
                : null,
          ),
        ),
        initialLibraryViewPreferencesProvider.overrideWithValue(
          LibraryViewPreferences(layoutShape: shape),
        ),
      ]),
      child: const AmeApp(),
    ),
  );
  await tester.pumpAndSettle();
  return catalog;
}

class _Catalog with LibraryQuerySnapshotFixture implements LibraryCatalog {
  final requests = <LibraryQueryAnchor?>[];
  final assets = List.generate(180, _asset);
  Future<void>? pendingRead;
  Future<void>? pendingManifest;

  LibraryGalleryLayoutManifest manifest(LibraryGalleryQuery query) {
    final result = snapshot(query).snapshot;
    final count = result.assets.length;
    final builder = LibraryGalleryLayoutManifestBuilder(
      revision: result.revision,
      queryId: result.queryId,
      totalItems: count,
    );
    builder.append(
      LibraryGalleryLayoutManifestChunk(
        revision: result.revision,
        queryId: result.queryId,
        totalItems: count,
        startOrdinal: 0,
        locationIds: result.assets.map((asset) => asset.locationId).toList(),
        aspectRatioMilli: Uint16List.fromList(List.filled(count, 1333)),
        dateGroupIndices: Uint16List(count),
        dateGroups: const ["2020-09-02"],
        flags: Uint8List.fromList(
          List.filled(count, libraryGalleryLayoutDimensionsKnownFlag),
        ),
      ),
    );
    return builder.build();
  }

  LibraryQuerySnapshot snapshot(
    LibraryGalleryQuery query, [
    LibraryQueryAnchor? anchor,
  ]) {
    final retained = assets.where(
      (asset) => asset.locationId == anchor?.requestedLocationId,
    );
    final condition = query.metadataFilter.isActive
        ? query.metadataFilter.extensions.join(",")
        : query.searchText;
    final collection = query.collection;
    final category = collection is LibrarySmartAlbumCollection
        ? collection.categoryId
        : null;
    final filtered = switch (category ?? condition) {
      "drawing" => assets.take(150).toList(),
      "photo" => assets.skip(150).toList(),
      "one" => retained.toList(),
      "bottom" => [...assets.take(24), ...retained],
      "excluded" => assets.take(24).toList(),
      "empty" => <LibraryAsset>[],
      _ => assets,
    };
    final index = filtered.indexWhere(
      (asset) => asset.locationId == anchor?.requestedLocationId,
    );
    final queryId = "filter-${category ?? condition}";
    return LibraryQuerySnapshot(
      snapshot: LibrarySnapshot(
        catalogPath: "controlled.sqlite3",
        revision: BigInt.one,
        queryId: queryId,
        roots: const [
          LibraryRoot(
            id: "root",
            path: "fixture",
            displayPath: "fixture",
            createdUnixMs: 1,
            assetCount: 180,
            issueCount: 0,
          ),
        ],
        assets: filtered,
        queryAnchorResolution: anchor == null
            ? null
            : LibraryQueryAnchorResolution(
                requestedLocationId: anchor.requestedLocationId,
                locationId: index < 0 ? null : filtered[index].locationId,
                ordinal: index < 0 ? null : index,
                windowStartItemOffset: 0,
              ),
      ),
      timeline: LibraryTimeline(
        revision: BigInt.one,
        queryId: queryId,
        totalItems: filtered.length,
        buckets: [
          if (filtered.isNotEmpty)
            LibraryTimeBucket(
              monthKey: "2020-09",
              itemCount: filtered.length,
              aspectRatioSum: filtered.length * 4 / 3,
            ),
        ],
      ),
    );
  }

  @override
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) async {
    requests.add(anchor);
    await pendingRead;
    return snapshot(query, anchor);
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async => snapshot(query).snapshot;

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      snapshot(query).timeline;

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) => throw UnimplementedError("This fixture does not navigate dates");

  @override
  Future<bool> unregisterRoot(String rootId) =>
      throw UnimplementedError("This fixture does not remove roots");
}

LibraryAsset _asset(int index) => LibraryAsset(
  assetId: "asset-$index",
  locationId: "location-$index",
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "fixture-$index.png",
  displayPath: "fixture-$index.png",
  relativePath: "fixture-$index.png",
  previewPath: "",
  fileSize: BigInt.one,
  modifiedUnixMs: DateTime.utc(2020, 9, 2).millisecondsSinceEpoch,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 400,
  height: 300,
);
