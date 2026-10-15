import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_filmstrip.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_filmstrip_slot.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("collapse preserves strip position and pauses preview demand", (
    tester,
  ) async {
    final assets = List.generate(60, _asset);
    final controller = _RetainedFilmstripController();
    final visible = ValueNotifier(false);
    final selection = ValueNotifier(assets[20]);
    addTearDown(visible.dispose);
    addTearDown(selection.dispose);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryControllerProvider.overrideWith(() => controller),
          initialLibraryStateProvider.overrideWithValue(
            LibraryState(assets: assets),
          ),
        ],
        child: MaterialApp(
          home: Scaffold(
            body: Center(
              child: SizedBox(
                width: 700,
                child: ListenableBuilder(
                  listenable: Listenable.merge([visible, selection]),
                  builder: (context, _) => LibraryViewerFilmstripSlot(
                    visible: visible.value,
                    child: LibraryViewerFilmstrip(
                      selection: selection.value,
                      onSelected: (asset) => selection.value = asset,
                      loadPrevious: () async {},
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(const Key("viewer-filmstrip"), skipOffstage: false),
      findsNothing,
    );
    expect(controller.demands, 0);
    visible.value = true;
    await tester.pumpAndSettle();
    final strip = find.byKey(const Key("viewer-filmstrip"));
    final scroll = tester.widget<ListView>(strip).controller!;
    expect(scroll.offset, 1700);
    final initialDemands = controller.demands;
    await tester.tap(find.byKey(const ValueKey("location-21")));
    await tester.pumpAndSettle();
    expect(selection.value.locationId, "location-21");
    expect(scroll.offset, 1700);
    scroll.jumpTo(3000);
    await tester.pumpAndSettle();
    visible.value = false;
    await tester.pumpAndSettle();
    final hiddenDemands = controller.demands;
    expect(hiddenDemands, greaterThan(initialDemands));
    expect(controller.releases, greaterThan(0));
    expect(strip, findsNothing);
    final hiddenStrip = find.byKey(
      const Key("viewer-filmstrip"),
      skipOffstage: false,
    );
    expect(tester.widget<ListView>(hiddenStrip).controller, same(scroll));
    await tester.pump(const Duration(seconds: 1));
    expect(controller.demands, hiddenDemands);
    controller.replaceAssets([
      assets.first.withPreview(
        previewPath: "unused/hidden-preview.jpg",
        width: 640,
        height: 480,
        previewStatus: LibraryPreviewStatus.ready,
      ),
    ]);
    await tester.pumpAndSettle();
    expect(
      tester.widget<ListView>(hiddenStrip).semanticChildCount,
      assets.length,
    );
    expect(find.byType(Image, skipOffstage: false), findsNothing);
    controller.replaceAssets(assets);
    visible.value = true;
    await tester.pumpAndSettle();
    expect(tester.widget<ListView>(strip).controller, same(scroll));
    expect(scroll.offset, 3000);

    visible.value = false;
    await tester.pumpAndSettle();
    selection.value = assets[45];
    await tester.pumpAndSettle();
    visible.value = true;
    await tester.pumpAndSettle();
    expect(
      find.byKey(const ValueKey("location-45")).hitTestable(),
      findsOneWidget,
    );
    expect(scroll.offset, 3900);
    final beforeDispose = controller.releases;
    await tester.pumpWidget(const SizedBox());
    expect(controller.releases, beforeDispose + 1);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "large filmstrip builds visible thumbnails, centers selection and selects neighbors",
    (tester) async {
      final assets = List.generate(10000, _asset);
      LibraryAsset? selected;
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState(assets: assets),
            ),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: Center(
                child: SizedBox(
                  height: 104,
                  width: 700,
                  child: LibraryViewerFilmstrip(
                    selection: assets[5000],
                    onSelected: (asset) => selected = asset,
                    loadPrevious: () async {},
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(
        find
            .descendant(
              of: find.byKey(const Key("viewer-filmstrip")),
              matching: find.byType(InkWell),
            )
            .evaluate()
            .length,
        lessThanOrEqualTo(11),
      );
      final current = find.byKey(const ValueKey("location-5000"));
      expect(current.hitTestable(), findsOneWidget);
      final strip = find.byKey(const Key("viewer-filmstrip"));
      expect(
        tester.getCenter(current).dx,
        closeTo(tester.getCenter(strip).dx, 0.1),
      );
      await tester.tap(find.byKey(const ValueKey("location-5001")));
      expect(selected?.locationId, "location-5001");
      expect(find.byKey(const ValueKey("location-0")), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  for (final reducedMotion in [false, true]) {
    testWidgets(
      "end buttons scroll thumbnails without selecting; reducedMotion=$reducedMotion",
      (tester) async {
        final assets = List.generate(20, _asset);
        LibraryAsset? selected;
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              initialLibraryStateProvider.overrideWithValue(
                LibraryState(assets: assets),
              ),
            ],
            child: MaterialApp(
              home: MediaQuery(
                data: MediaQueryData(disableAnimations: reducedMotion),
                child: Scaffold(
                  body: Center(
                    child: SizedBox(
                      height: 104,
                      width: 700,
                      child: LibraryViewerFilmstrip(
                        selection: assets.first,
                        onSelected: (asset) => selected = asset,
                        loadPrevious: () async {},
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final strip = find.byKey(const Key("viewer-filmstrip"));
        final scroll = tester.widget<ListView>(strip).controller!;
        final left = find.byKey(const Key("viewer-filmstrip-scroll-left"));
        final right = find.byKey(const Key("viewer-filmstrip-scroll-right"));
        expect(tester.widget<IconButton>(left).onPressed, isNull);
        expect(tester.widget<IconButton>(right).onPressed, isNotNull);
        await tester.tap(right);
        if (reducedMotion) expect(scroll.offset, 100);
        await tester.pumpAndSettle();
        expect(scroll.offset, 100);
        expect(tester.widget<IconButton>(left).onPressed, isNotNull);
        expect(selected, isNull);
        await tester.tap(left);
        await tester.pumpAndSettle();
        expect(scroll.offset, 0);
        expect(tester.widget<IconButton>(left).onPressed, isNull);
        scroll.jumpTo(scroll.position.maxScrollExtent);
        await tester.pumpAndSettle();
        expect(tester.widget<IconButton>(right).onPressed, isNull);
        expect(tester.widget<IconButton>(left).onPressed, isNotNull);
        expect(selected, isNull);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets("filmstrip waits for query refresh before requesting a page", (
    tester,
  ) async {
    final assets = List.generate(20, _asset);
    final controller = _PagingFilmstripController();
    var previousRequests = 0;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryControllerProvider.overrideWith(() => controller),
          initialLibraryStateProvider.overrideWithValue(
            LibraryState(
              assets: assets,
              windowStartItemOffset: 20,
              queryActivity: const LibraryQueryLoading(LibraryGalleryQuery()),
              previousCursor: LibraryCatalogCursor(
                revision: BigInt.one,
                queryId: "filmstrip",
                primaryMissing: false,
                primaryText: "",
                primaryNumber: 0,
                rootId: "root",
                locationId: assets.first.locationId,
              ),
            ),
          ),
        ],
        child: MaterialApp(
          home: Scaffold(
            body: Center(
              child: SizedBox(
                height: 104,
                width: 700,
                child: LibraryViewerFilmstrip(
                  selection: assets.first,
                  onSelected: (_) {},
                  loadPrevious: () async {
                    previousRequests += 1;
                    if (!controller.libraryState.isRefreshingQuery) {
                      controller.finishPaging();
                    }
                  },
                ),
              ),
            ),
          ),
        ),
      ),
    );
    for (var frame = 0; frame < 5; frame++) {
      await tester.pump(const Duration(milliseconds: 16));
    }
    expect(previousRequests, 0);
    expect(find.byType(CircularProgressIndicator), findsNothing);
    await tester.pumpAndSettle();
    expect(tester.binding.hasScheduledFrame, isFalse);

    controller.finishRefresh();
    await tester.pumpAndSettle();
    expect(previousRequests, 1);
    expect(tester.takeException(), isNull);
  });
}

class _PagingFilmstripController extends LibraryController {
  void finishRefresh() =>
      state = state.copyWith(queryActivity: const LibraryQueryIdle());

  void finishPaging() => state = state.copyWith(previousCursor: null);
}

class _RetainedFilmstripController extends LibraryController {
  int demands = 0;
  int releases = 0;

  void replaceAssets(List<LibraryAsset> assets) =>
      state = state.copyWith(assets: assets);

  @override
  void updateFilmstripPreviewDemand(
    Object owner,
    Iterable<LibraryAsset> visible,
    int previewEdge,
  ) {
    demands++;
    super.updateFilmstripPreviewDemand(owner, visible, previewEdge);
  }

  @override
  void releaseFilmstripPreviewDemand(Object owner) {
    releases++;
    super.releaseFilmstripPreviewDemand(owner);
  }
}

LibraryAsset _asset(int index) => LibraryAsset(
  assetId: "asset-$index",
  locationId: "location-$index",
  rootId: "root",
  activeScanId: "scan",
  sourcePath: "unused/photo-$index.jpg",
  displayPath: "photo-$index.jpg",
  relativePath: "photo-$index.jpg",
  previewPath: "",
  fileSize: BigInt.from(2000 + index),
  modifiedUnixMs: index,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 640,
  height: 480,
  previewStatus: LibraryPreviewStatus.failed,
);
