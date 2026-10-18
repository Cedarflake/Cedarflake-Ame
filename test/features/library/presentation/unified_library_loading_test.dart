import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_query_snapshot_fixture.dart";
import "support/library_screen_fixture.dart";

void main() {
  testWidgets("reuses the gallery loading bar for a distant visible range", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            populatedScreenState(
              totalItems: 64,
            ).copyWith(isLoadingVisibleRange: true),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final loadingBar = find.byKey(const Key("library-top-loading"));
    expect(loadingBar, findsOneWidget);
    expect(tester.widget<LinearProgressIndicator>(loadingBar).minHeight, 2);
    expect(
      find.byKey(const Key("library-visible-range-loading")),
      findsNothing,
    );
  });

  testWidgets(
    "square direct jump shows the top loading line before its manifest",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = populatedScreenState(totalItems: 4000);
      final snapshot = LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: initialState.catalogRevision ?? BigInt.zero,
        queryId: initialState.queryId,
        roots: initialState.roots,
        assets: initialState.assets,
      );
      final catalog = _BlockingTimeQueryCatalog(
        snapshot: snapshot,
        timeline: initialState.timeline!,
      );
      final manifestLoader = ControlledLayoutManifestLoader();
      addTearDown(catalog.complete);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            initialLibraryViewPreferencesProvider.overrideWithValue(
              const LibraryViewPreferences(
                layoutShape: GalleryLayoutShape.square,
              ),
            ),
            libraryCatalogProvider.overrideWithValue(catalog),
            libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(
              manifestLoader,
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump();

      final scrollPosition = galleryScrollPosition(tester);
      scrollPosition.jumpTo(scrollPosition.maxScrollExtent * 0.8);
      await tester.pump();

      expect(catalog.timeAnchors, isNotEmpty);
      expect(catalog.timeAnchors.last.itemOffset, greaterThan(80));
      final loadingBar = find.byKey(const Key("library-top-loading"));
      expect(loadingBar, findsOneWidget);
      expect(tester.widget<LinearProgressIndicator>(loadingBar).minHeight, 2);
      expect(
        find.byKey(const Key("library-visible-range-loading")),
        findsNothing,
      );
    },
  );

  testWidgets(
    "shows the restored top loading line on the first unloaded timeline frame",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = populatedScreenState(totalItems: 120);
      final catalog = _BlockingTimeQueryCatalog(
        snapshot: LibrarySnapshot(
          catalogPath: initialState.catalogPath ?? "",
          revision: initialState.catalogRevision ?? BigInt.zero,
          queryId: initialState.queryId,
          roots: initialState.roots,
          assets: initialState.assets,
        ),
        timeline: initialState.timeline!,
      );
      addTearDown(catalog.complete);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump();

      final slider = tester.widget<Slider>(
        find.byKey(const Key("timeline-slider")),
      );
      slider.onChangeStart?.call(slider.value);
      slider.onChanged?.call(0.2);
      slider.onChangeEnd?.call(0.2);
      await tester.pump();

      expect(catalog.timeAnchors, hasLength(1));
      expect(catalog.timeAnchors.single.itemOffset, greaterThan(0));
      final loadingBar = find.byKey(const Key("library-top-loading"));
      expect(loadingBar, findsOneWidget);
      expect(tester.widget<LinearProgressIndicator>(loadingBar).minHeight, 2);
      expect(
        find.byKey(const Key("library-visible-range-loading")),
        findsNothing,
      );

      catalog.complete();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 250));
    },
  );

  testWidgets("uses one top line for scroll-triggered page loading", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final state = populatedScreenState(
      totalItems: 2,
    ).copyWith(isLoadingPage: true, isLoadingPreviousPage: true);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [initialLibraryStateProvider.overrideWithValue(state)],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    expect(find.byKey(const Key("library-top-loading")), findsOneWidget);
    expect(
      find.descendant(
        of: find.byKey(const Key("library-photo-wall")),
        matching: find.byType(CircularProgressIndicator),
      ),
      findsNothing,
    );
  });

  testWidgets(
    "keeps keyset continuation automatic without visible pagination",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final fallbackUnixMs = DateTime(2024, 3, 4).millisecondsSinceEpoch;
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [
          LibraryRoot(
            id: "root-1",
            path: "C:\\Pictures",
            displayPath: "C:\\Pictures",
            activeScanId: "scan-1",
            createdUnixMs: 1,
            assetCount: 2,
            issueCount: 0,
          ),
        ],
        assets: [
          LibraryAsset(
            assetId: "asset-1",
            locationId: "location-1",
            rootId: "root-1",
            activeScanId: "scan-1",
            sourceRevision: null,
            sourceGeneration: BigInt.one,
            sourcePath: "C:\\Pictures\\one.png",
            displayPath: "C:\\Pictures\\one.png",
            relativePath: "one.png",
            previewPath: "C:\\Missing\\one.jpg",
            fileSize: BigInt.one,
            modifiedUnixMs: fallbackUnixMs,
            width: 1,
            height: 1,
          ),
        ],
        nextCursor: LibraryCatalogCursor(
          revision: BigInt.one,
          queryId: "query-1",
          primaryMissing: true,
          primaryText: "",
          primaryNumber: 1,
          rootId: "root-1",
          locationId: "location-1",
        ),
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState.fromSnapshot(snapshot),
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();

      expect(find.byKey(const Key("library-load-more-button")), findsNothing);
      expect(find.byKey(const Key("gallery-date-2024-03-04")), findsOneWidget);
      final summary = tester.widget<Text>(
        find.byKey(const Key("library-summary")),
      );
      expect(summary.data, "2 张图片");
    },
  );
}

class _BlockingTimeQueryCatalog
    with LibraryQuerySnapshotFixture
    implements LibraryCatalog {
  _BlockingTimeQueryCatalog({required this.snapshot, required this.timeline});

  final LibrarySnapshot snapshot;
  final LibraryTimeline timeline;
  final List<LibraryTimeAnchor> timeAnchors = [];
  final Completer<LibrarySnapshot> _timeResult = Completer();

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async => snapshot;

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) {
    timeAnchors.add(anchor);
    return _timeResult.future;
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      timeline;

  @override
  Future<bool> unregisterRoot(String rootId) async => true;

  void complete() {
    if (!_timeResult.isCompleted) {
      _timeResult.complete(snapshot);
    }
  }
}
