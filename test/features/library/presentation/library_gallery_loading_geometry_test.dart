import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("loading feedback preserves the populated gallery viewport", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final controller = _LoadingFeedbackController();
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(_populatedState()),
        libraryControllerProvider.overrideWith(() => controller),
      ],
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
      expect(loadingBar, findsOneWidget);
      expect(tester.getRect(wall), idleBounds, reason: activity.name);
      expect(position.viewportDimension, idleExtent, reason: activity.name);
      expect(position.pixels, idleOffset, reason: activity.name);
      expect(tester.getRect(tile), idleTileBounds, reason: activity.name);
      expect(
        tester.widget<LinearProgressIndicator>(loadingBar).semanticsLabel,
        "正在加载图片",
      );

      controller.showLoading(null);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 50));
      expect(loadingBar, findsNothing);
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
