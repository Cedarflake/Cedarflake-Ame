import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_filmstrip.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
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
