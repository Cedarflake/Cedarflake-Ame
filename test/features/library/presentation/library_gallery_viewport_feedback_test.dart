import "package:cedarflake_ame/features/library/presentation/gallery_selection.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_activity.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_viewport_feedback.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/gallery_wall_fixture.dart";

void main() {
  for (final scenario in [
    (
      direction: LibraryPreviewMovementDirection.forward,
      near: [207, 208, 209],
      guard: [203, 202],
    ),
    (
      direction: LibraryPreviewMovementDirection.backward,
      near: [203, 202, 201],
      guard: [207],
    ),
    (
      direction: LibraryPreviewMovementDirection.idle,
      near: [207, 208, 209],
      guard: [203, 202, 207],
    ),
  ]) {
    test(
      "${scenario.direction} keeps center-first bounded preview priorities",
      () {
        final state = galleryBoundaryState(
          itemCount: 1000,
          windowStart: 200,
          windowEnd: 212,
          hasPrevious: true,
          hasNext: true,
        );
        final metrics = LibraryGalleryLayoutMetrics(
          contentExtent: 1200,
          photoRowHeight: 100,
          dateAnchors: const [],
          locationOffsets: const {},
          itemOffsets: [for (var index = 0; index < 12; index++) index * 100.0],
          itemIndexBase: 200,
        );
        final controller = RecordingGalleryController();
        LibraryGalleryViewportFeedback.updatePreviewDemand(
          controller: controller,
          state: state,
          layoutMetrics: metrics,
          scrollOffset: 400,
          viewportDimension: 200,
          direction: scenario.direction,
          anchorGlobalItemIndex: 205,
          previewEdgeFor: (_, _) => 256,
        );
        final demand = controller.previewDemandRequests.single;
        expect(demand.visible, [
          "location-205",
          "location-204",
          "location-206",
        ]);
        expect(demand.nearDirection, [
          for (final item in scenario.near) "location-$item",
        ]);
        expect(demand.guard, [
          for (final item in scenario.guard) "location-$item",
        ]);
        final range = LibraryGalleryViewportFeedback.visibleRangeFor(
          layoutMetrics: metrics,
          scrollOffset: 400,
          viewportDimension: 200,
          queryId: state.queryId,
          revision: state.catalogRevision,
        );
        expect(range?.startGlobalItemIndex, 204);
        expect(range?.endGlobalItemIndexExclusive, 207);
      },
    );
  }

  for (final withManifest in [false, true]) {
    testWidgets(
      "hidden gallery retires preview work and resumes ($withManifest)",
      (tester) async {
        final state = galleryBoundaryState(
          itemCount: 40,
          windowStart: 0,
          windowEnd: 40,
          hasPrevious: false,
          hasNext: false,
        );
        final controller = RecordingGalleryController();
        final scroll = ScrollController();
        addTearDown(scroll.dispose);
        final active = ValueNotifier(true);
        addTearDown(active.dispose);
        await tester.pumpWidget(
          ProviderScope(
            child: MaterialApp(
              home: ValueListenableBuilder<bool>(
                valueListenable: active,
                builder: (context, enabled, _) => Visibility(
                  visible: enabled,
                  maintainState: true,
                  child: LibraryGalleryActivity(
                    controller: controller,
                    child: LibraryGalleryWall(
                      state: state,
                      controller: controller,
                      scrollController: scroll,
                      layoutShape: GalleryLayoutShape.equalHeight,
                      thumbnailSize: GalleryThumbnailSize.medium,
                      selection: GallerySelection.empty(state.queryId),
                      isSelecting: false,
                      layoutManifest: withManifest ? galleryManifest(40) : null,
                      onOpen: (_) {},
                      onToggleSelection: (_) {},
                      onViewInformation: (_) {},
                      onCopyPath: (_) {},
                      onRevealFile: (_) {},
                      onVisiblePositionChanged: (_) {},
                      onLoadPrevious: () async {},
                      onLayoutChanged: (_, _) {},
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pump();
        expect(controller.previewDemandRequests.last.visible, isNotEmpty);
        final originalPosition = scroll.position;
        active.value = false;
        await tester.pump();
        expect(controller.previewDemandRequests.last.visible, isEmpty);
        final hiddenCount = controller.previewDemandRequests.length;
        // A late scroll notification must not repopulate hidden preview demand.
        scroll.jumpTo(100);
        await tester.pump();
        expect(controller.previewDemandRequests.length, hiddenCount);
        active.value = true;
        await tester.pump();
        expect(identical(scroll.position, originalPosition), isTrue);
        expect(controller.previewDemandRequests.last.visible, isNotEmpty);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }

  test("visible range excludes surrounding prefetch evidence", () {
    final range = LibraryGalleryVisibleRange(
      queryId: "query-1",
      revision: BigInt.one,
      startGlobalItemIndex: 20,
      endGlobalItemIndexExclusive: 30,
    );

    expect(range.containsGlobalItemIndex(19), isFalse);
    expect(range.containsGlobalItemIndex(20), isTrue);
    expect(range.containsGlobalItemIndex(29), isTrue);
    expect(range.containsGlobalItemIndex(30), isFalse);
    expect(
      range.contains(
        queryId: "stale-query",
        revision: BigInt.one,
        globalItemIndex: 24,
      ),
      isFalse,
    );
  });

  testWidgets("publishes the initial visible range before a manifest arrives", (
    tester,
  ) async {
    final state = galleryBoundaryState(
      itemCount: 40,
      windowStart: 0,
      windowEnd: 40,
      hasPrevious: false,
      hasNext: false,
    );
    final controller = RecordingGalleryController();
    final scrollController = ScrollController();
    LibraryGalleryVisibleRange? visibleRange;
    addTearDown(scrollController.dispose);

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: SizedBox(
            width: 900,
            height: 600,
            child: LibraryGalleryWall(
              state: state,
              controller: controller,
              scrollController: scrollController,
              layoutShape: GalleryLayoutShape.equalHeight,
              thumbnailSize: GalleryThumbnailSize.medium,
              selection: GallerySelection.empty(state.queryId),
              isSelecting: false,
              onOpen: (_) {},
              onToggleSelection: (_) {},
              onViewInformation: (_) {},
              onCopyPath: (_) {},
              onRevealFile: (_) {},
              onVisiblePositionChanged: (_) {},
              onVisibleRangeChanged: (range) {
                visibleRange = range;
              },
              onLoadPrevious: () async {},
              onLayoutChanged: (_, _) {},
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    expect(visibleRange, isNotNull);
    expect(visibleRange!.queryId, state.queryId);
    expect(visibleRange!.revision, state.catalogRevision);
    expect(visibleRange!.containsGlobalItemIndex(0), isTrue);
    expect(controller.previewDemandRequests, isNotEmpty);
  });

  testWidgets(
    "square direct jump requests the unloaded range before a manifest arrives",
    (tester) async {
      final state = galleryBoundaryState(
        itemCount: 4000,
        windowStart: 0,
        windowEnd: 80,
        hasPrevious: false,
        hasNext: true,
      );
      final controller = RecordingGalleryController();
      final scrollController = ScrollController();
      addTearDown(scrollController.dispose);

      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: SizedBox(
              width: 900,
              height: 600,
              child: LibraryGalleryWall(
                state: state,
                controller: controller,
                scrollController: scrollController,
                layoutShape: GalleryLayoutShape.square,
                thumbnailSize: GalleryThumbnailSize.medium,
                selection: GallerySelection.empty(state.queryId),
                isSelecting: false,
                onOpen: (_) {},
                onToggleSelection: (_) {},
                onViewInformation: (_) {},
                onCopyPath: (_) {},
                onRevealFile: (_) {},
                onVisiblePositionChanged: (_) {},
                onVisibleRangeChanged: (_) {},
                onLoadPrevious: () async {},
                onLayoutChanged: (_, _) {},
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      controller.visibleRangeRequests.clear();

      scrollController.jumpTo(scrollController.position.maxScrollExtent * 0.8);
      await tester.pump();

      expect(controller.visibleRangeRequests, isNotEmpty);
      expect(controller.visibleRangeRequests.last.start, greaterThan(80));
      expect(
        find.descendant(
          of: find.byKey(const Key("library-trailing-placeholder")),
          matching: find.byType(CustomPaint),
        ),
        findsNothing,
      );
    },
  );
}
