import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/presentation/ame_loading_indicator.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/application_test_bindings.dart";

void main() {
  testWidgets(
    "published empty favorites stay visible through query refreshes",
    (tester) async {
      final controller = _LoadingFeedbackController();
      final favorites = _populatedState().copyWith(
        assets: const [],
        timeline: null,
        query: const LibraryGalleryQuery(
          collection: LibraryGalleryCollection.favorites(),
        ),
      );
      final container = ProviderContainer(
        overrides: applicationTestOverrides([
          initialLibraryStateProvider.overrideWithValue(favorites),
          libraryControllerProvider.overrideWith(() => controller),
        ]),
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await tester.pumpAndSettle();
      final title = find.text("收藏夹里还没有图片");
      final bounds = tester.getRect(title);
      final element = tester.element(title);
      const albumQuery = LibraryGalleryQuery(
        collection: LibraryGalleryCollection.album("one"),
      );
      final backgroundRefresh = favorites.copyWith(isLoadingTimeline: true);
      for (final pending in [
        backgroundRefresh,
        backgroundRefresh,
        favorites.copyWith(
          queryActivity: const LibraryQueryLoading(albumQuery),
          isLoadingTimeline: true,
        ),
      ]) {
        controller.publish(pending);
        await tester.pump();
        for (final elapsed in [100, 600]) {
          await tester.pump(Duration(milliseconds: elapsed));
          expect(tester.element(title), same(element));
          expect(tester.getRect(title), bounds);
          expect(find.byType(AmeLoadingIndicator), findsNothing);
          expect(find.text("这个相册里还没有图片"), findsNothing);
        }
        controller.publish(favorites);
        await tester.pumpAndSettle();
        expect(tester.element(title), same(element));
        expect(tester.getRect(title), bounds);
      }
      controller.publish(favorites.copyWith(query: albumQuery));
      await tester.pumpAndSettle();
      expect(title, findsNothing);
      expect(find.text("这个相册里还没有图片"), findsOneWidget);
      controller.publish(_populatedState());
      await tester.pumpAndSettle();
      expect(find.text("这个相册里还没有图片"), findsNothing);
      expect(find.byType(LibraryPhotoTile), findsWidgets);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets("loading feedback preserves the populated gallery viewport", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final semantics = tester.ensureSemantics();
    try {
      final controller = _LoadingFeedbackController();
      final container = ProviderContainer(
        overrides: applicationTestOverrides([
          initialLibraryStateProvider.overrideWithValue(_populatedState()),
          libraryControllerProvider.overrideWith(() => controller),
        ]),
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await tester.pump();
      await tester.pump();

      final wall = find.byKey(const Key("library-photo-wall"));
      final scrollable = find.descendant(
        of: wall,
        matching: find.byType(Scrollable),
      );
      final position = tester.state<ScrollableState>(scrollable).position;
      expect(position.maxScrollExtent, greaterThan(0));
      position.jumpTo(position.maxScrollExtent / 2);
      await tester.pump();
      await tester.pump();
      final idleBounds = tester.getRect(wall);
      final idleOffset = position.pixels;
      final idleExtent = position.viewportDimension;
      final visibleTile = tester
          .widgetList<LibraryPhotoTile>(find.byType(LibraryPhotoTile))
          .firstWhere((tile) {
            final bounds = tester.getRect(find.byWidget(tile));
            return idleBounds.contains(bounds.center);
          });
      final tile = find.byKey(ValueKey(visibleTile.asset.locationId));
      final idleTileBounds = tester.getRect(tile);
      final loadingBar = find.byKey(const Key("library-top-loading"));

      for (final activity in _LoadingActivity.values) {
        controller.showLoading(activity);
        await tester.pump();
        expect(find.semantics.byLabel(RegExp("正在加载图片")), findsNothing);
        await tester.pump(const Duration(milliseconds: 250));
        await tester.pump(const Duration(milliseconds: 500));
        expect(loadingBar, findsOneWidget);
        expect(tester.getRect(wall), idleBounds, reason: activity.name);
        expect(position.viewportDimension, idleExtent, reason: activity.name);
        expect(position.pixels, idleOffset, reason: activity.name);
        expect(tester.getRect(tile), idleTileBounds, reason: activity.name);
        expect(
          find.semantics.byLabel(RegExp("正在加载图片")),
          findsOne,
          reason: activity.name,
        );

        controller.showLoading(null);
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 50));
        expect(find.semantics.byLabel(RegExp("正在加载图片")), findsNothing);
        await tester.pump(const Duration(milliseconds: 500));
        expect(
          tester.state<ScrollableState>(scrollable).position,
          same(position),
        );
        expect(tester.getRect(wall), idleBounds, reason: activity.name);
        expect(position.viewportDimension, idleExtent, reason: activity.name);
        expect(position.pixels, idleOffset, reason: activity.name);
        expect(tester.getRect(tile), idleTileBounds, reason: activity.name);
      }
      await tester.pumpWidget(const SizedBox.shrink());
    } finally {
      semantics.dispose();
    }
  });
}

enum _LoadingActivity {
  query,
  nextPage,
  previousPage,
  timeAnchor,
  visibleRange,
}

class _LoadingFeedbackController extends LibraryController {
  void publish(LibraryState value) => state = value;

  @override
  void ensureVisibleRange({
    required int startItemOffset,
    required int endItemOffsetExclusive,
  }) {
    // This fixture owns each loading interval until showLoading completes it.
  }

  void showLoading(_LoadingActivity? activity) {
    state = state.copyWith(
      queryActivity: activity == _LoadingActivity.query
          ? LibraryQueryLoading(state.query)
          : const LibraryQueryIdle(),
      isLoadingPage: activity == _LoadingActivity.nextPage,
      isLoadingPreviousPage: activity == _LoadingActivity.previousPage,
      isLoadingTimeAnchor: activity == _LoadingActivity.timeAnchor,
      isLoadingVisibleRange: activity == _LoadingActivity.visibleRange,
    );
  }
}

LibraryState _populatedState() {
  const count = 80;
  final snapshot = LibrarySnapshot(
    catalogPath: "C:\\AmeFixture\\catalog.sqlite3",
    revision: BigInt.one,
    queryId: "loading-geometry",
    roots: [
      LibraryRoot(
        id: "root",
        path: "C:\\AmeFixture\\images",
        displayPath: "C:\\AmeFixture\\images",
        activeScanId: "scan",
        createdUnixMs: 1,
        assetCount: count,
        issueCount: 0,
        availability: LibraryRootAvailability.available,
      ),
    ],
    assets: [
      for (var index = 0; index < count; index++)
        LibraryAsset(
          assetId: "asset-$index",
          locationId: "location-$index",
          rootId: "root",
          activeScanId: "scan",
          sourceRevision: null,
          sourceGeneration: BigInt.one,
          sourcePath: "C:\\AmeFixture\\images\\$index.png",
          displayPath: "C:\\AmeFixture\\images\\$index.png",
          relativePath: "$index.png",
          previewPath: "C:\\AmeFixture\\previews\\$index.jpg",
          fileSize: BigInt.one,
          modifiedUnixMs: 1,
          width: 4,
          height: 3,
        ),
    ],
  );
  return LibraryState.fromSnapshot(snapshot).copyWith(
    timeline: LibraryTimeline(
      revision: BigInt.one,
      queryId: snapshot.queryId,
      totalItems: count,
      buckets: const [LibraryTimeBucket(itemCount: count, aspectRatioSum: 80)],
    ),
  );
}
