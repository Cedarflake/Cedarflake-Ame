import "dart:async";

import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_time_navigation.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final shape in GalleryLayoutShape.values) {
    testWidgets("late date navigation keeps the heading visible in $shape", (
      tester,
    ) async {
      final controller = ScrollController();
      addTearDown(controller.dispose);
      final seek = Completer<bool>();
      var loaded = false;
      final timeline = LibraryTimeline(
        revision: BigInt.one,
        queryId: "query",
        totalItems: 6,
        buckets: const [
          LibraryTimeBucket(
            monthKey: "2026-10",
            itemCount: 3,
            aspectRatioSum: 3,
          ),
          LibraryTimeBucket(
            monthKey: "2025-01",
            itemCount: 3,
            aspectRatioSum: 3,
          ),
        ],
      );
      LibraryGalleryLayoutMetrics metrics() => LibraryGalleryLayoutMetrics(
        contentExtent: 1000,
        photoRowHeight: 200,
        dateAnchors: [
          const LibraryGalleryDateAnchor(
            id: "2026-10-01",
            label: "October",
            scrollOffset: 18,
            year: 2026,
            isUnknown: false,
          ),
          if (loaded)
            const LibraryGalleryDateAnchor(
              id: "2025-01-01",
              label: "January",
              scrollOffset: 350,
              year: 2025,
              isUnknown: false,
            ),
        ],
        locationOffsets: const {},
        itemOffsets: loaded
            ? const [50, 50, 50, 400, 800, 800]
            : const [50, 50, 50],
        isQueryWide: loaded,
      );
      Widget page() => MaterialApp(
        home: Scaffold(
          body: SizedBox(
            height: 300,
            child: Row(
              children: [
                Expanded(
                  child: ListView(
                    controller: controller,
                    padding: EdgeInsets.zero,
                    children: const [
                      SizedBox(height: 350),
                      SizedBox(height: 50, child: Text("2025年1月1日")),
                      SizedBox(height: 600),
                    ],
                  ),
                ),
                LibraryTimeNavigation(
                  isLoading: !loaded,
                  scrollController: controller,
                  layoutMetrics: metrics(),
                  timeline: timeline,
                  layoutShape: shape,
                  windowStartItemOffset: 0,
                  loadedItemCount: loaded ? 6 : 3,
                  onSeek: (bucket, offset) {
                    expect(bucket.monthKey, "2025-01");
                    expect(offset, 0);
                    return seek.future;
                  },
                ),
              ],
            ),
          ),
        ),
      );
      await tester.pumpWidget(page());
      await tester.pumpWidget(page());
      final slider = tester.widget<Slider>(
        find.byKey(const Key("timeline-slider")),
      );
      slider.onChangeStart?.call(0.5);
      slider.onChanged?.call(0.5);
      slider.onChangeEnd?.call(0.5);
      await tester.pump();
      loaded = true;
      await tester.pumpWidget(page());
      seek.complete(true);
      await tester.pumpAndSettle();
      expect(controller.offset, 350);
      final viewport = tester.getRect(find.byType(ListView));
      final heading = tester.getRect(find.text("2025年1月1日"));
      expect(heading.top, greaterThanOrEqualTo(viewport.top));
      expect(heading.bottom, lessThanOrEqualTo(viewport.bottom));
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }
  for (final scenario in [
    (name: "loaded rows", isDeferred: false, scrollWhileLoading: false),
    (name: "late details", isDeferred: true, scrollWhileLoading: false),
    (name: "scroll during loading", isDeferred: true, scrollWhileLoading: true),
  ]) {
    testWidgets(
      "rail retains the exact scroll position with ${scenario.name}",
      (tester) async {
        final controller = ScrollController();
        addTearDown(controller.dispose);
        final seek = Completer<bool>();
        var loadedItemCount = scenario.isDeferred ? 0 : 6;
        final timeline = LibraryTimeline(
          revision: BigInt.one,
          queryId: "query",
          totalItems: 6,
          buckets: const [
            LibraryTimeBucket(
              monthKey: "2026-10",
              itemCount: 3,
              aspectRatioSum: 90,
            ),
            LibraryTimeBucket(
              monthKey: "2025-01",
              itemCount: 3,
              aspectRatioSum: 10,
            ),
          ],
        );
        final metrics = LibraryGalleryLayoutMetrics(
          contentExtent: 1000,
          photoRowHeight: 200,
          dateAnchors: const [
            LibraryGalleryDateAnchor(
              id: "2026-10-01",
              label: "October",
              scrollOffset: 18,
              year: 2026,
              isUnknown: false,
            ),
            LibraryGalleryDateAnchor(
              id: "2025-01-01",
              label: "January",
              scrollOffset: 350,
              year: 2025,
              isUnknown: false,
            ),
          ],
          locationOffsets: const {},
          itemOffsets: const [50, 50, 50, 400, 800, 800],
          isQueryWide: true,
        );
        Widget page() => MaterialApp(
          home: Scaffold(
            body: SizedBox(
              height: 300,
              child: Row(
                children: [
                  Expanded(
                    child: ListView(
                      controller: controller,
                      padding: EdgeInsets.zero,
                      children: const [SizedBox(height: 1000)],
                    ),
                  ),
                  LibraryTimeNavigation(
                    isLoading: false,
                    scrollController: controller,
                    layoutMetrics: metrics,
                    timeline: timeline,
                    layoutShape: GalleryLayoutShape.equalHeight,
                    windowStartItemOffset: 0,
                    loadedItemCount: loadedItemCount,
                    onSeek: (_, _) => seek.future,
                  ),
                ],
              ),
            ),
          ),
        );
        await tester.pumpWidget(page());
        await tester.pumpWidget(page());
        expect(controller.position.maxScrollExtent, 700);
        final slider = tester.widget<Slider>(
          find.byKey(const Key("timeline-slider")),
        );
        const rawSliderValue = 1 - 0.61;
        slider.onChangeStart?.call(rawSliderValue);
        slider.onChanged?.call(rawSliderValue);
        await tester.pump();
        expect(controller.offset, closeTo(427, 0.001));
        slider.onChangeEnd?.call(rawSliderValue);
        await tester.pump();
        expect(controller.offset, closeTo(427, 0.001));
        if (scenario.scrollWhileLoading) {
          controller.jumpTo(310);
          await tester.pump();
        }
        loadedItemCount = 6;
        await tester.pumpWidget(page());
        seek.complete(true);
        await tester.pump();
        await tester.pump();
        expect(
          controller.offset,
          closeTo(scenario.scrollWhileLoading ? 310 : 427, 0.001),
        );
        controller.jumpTo(350);
        await tester.pump();
        expect(
          tester.widget<Slider>(find.byKey(const Key("timeline-slider"))).value,
          closeTo(0.5, 0.000001),
        );
        await tester.drag(find.byType(ListView), const Offset(0, 1000));
        await tester.pumpAndSettle();
        expect(controller.offset, 0);
        expect(
          tester.getCenter(find.byKey(const Key("timeline-current-line"))),
          tester.getCenter(find.byKey(const ValueKey("time-marker-2026-10"))),
        );
        await tester.drag(find.byType(ListView), const Offset(0, -2000));
        await tester.pumpAndSettle();
        expect(controller.offset, controller.position.maxScrollExtent);
        expect(
          tester.getCenter(find.byKey(const Key("timeline-current-line"))),
          tester.getCenter(find.byKey(const Key("timeline-end-marker"))),
        );
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }
}
