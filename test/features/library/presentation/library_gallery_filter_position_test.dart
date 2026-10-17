import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_filter.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_query_snapshot_fixture.dart";

void main() {
  for (final shape in GalleryLayoutShape.values) {
    for (final filter in ["one", "bottom", "excluded", "empty"]) {
      testWidgets("$shape restores middle pixels after filter $filter", (
        tester,
      ) async {
        final catalog = await _openGallery(tester, shape);
        _scroll(tester).jumpTo(_scroll(tester).maxScrollExtent * 0.43);
        await tester.pumpAndSettle();
        final originalPixels = _scroll(tester).pixels;
        await _filter(tester, filter);
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
        await _filter(tester, "");
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

Future<void> _filter(WidgetTester tester, String text) async {
  final filter = tester.widget<LibraryGalleryFilter>(
    find.byType(LibraryGalleryFilter),
  );
  filter.onChanged(filter.query.copyWith(searchText: text));
  await tester.pumpAndSettle();
}

Future<_Catalog> _openGallery(
  WidgetTester tester,
  GalleryLayoutShape shape,
) async {
  tester.view.physicalSize = const Size(1280, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final catalog = _Catalog();
  final initial = catalog.snapshot(const LibraryGalleryQuery());
  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        libraryCatalogProvider.overrideWithValue(catalog),
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(
            initial.snapshot,
          ).copyWith(timeline: initial.timeline),
        ),
        initialLibraryViewPreferencesProvider.overrideWithValue(
          LibraryViewPreferences(layoutShape: shape),
        ),
      ],
      child: const AmeApp(),
    ),
  );
  await tester.pumpAndSettle();
  return catalog;
}

class _Catalog with LibraryQuerySnapshotFixture implements LibraryCatalog {
  final requests = <LibraryQueryAnchor?>[];
  final assets = List.generate(180, _asset);

  LibraryQuerySnapshot snapshot(
    LibraryGalleryQuery query, [
    LibraryQueryAnchor? anchor,
  ]) {
    final retained = assets.where(
      (asset) => asset.locationId == anchor?.requestedLocationId,
    );
    final filtered = switch (query.searchText) {
      "one" => retained.toList(),
      "bottom" => [...assets.take(24), ...retained],
      "excluded" => assets.take(24).toList(),
      "empty" => <LibraryAsset>[],
      _ => assets,
    };
    final index = filtered.indexWhere(
      (asset) => asset.locationId == anchor?.requestedLocationId,
    );
    final queryId = "filter-${query.searchText}";
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
