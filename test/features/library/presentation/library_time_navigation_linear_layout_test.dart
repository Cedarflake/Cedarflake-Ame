import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_time_navigation.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("rail seeks within a row and tracks the real scroll range", (
    tester,
  ) async {
    final controller = ScrollController();
    addTearDown(controller.dispose);
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
          scrollOffset: 0,
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
                loadedItemCount: 6,
                onSeek: (_, _) async => true,
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
    slider.onChangeStart?.call(0.61);
    slider.onChanged?.call(0.61);
    await tester.pump();
    expect(controller.offset, closeTo(427, 0.001));
    slider.onChangeEnd?.call(0.61);
    await tester.pump();
    expect(controller.offset, closeTo(427, 0.001));
    controller.jumpTo(350);
    await tester.pump();
    expect(
      tester.widget<Slider>(find.byKey(const Key("timeline-slider"))).value,
      closeTo(0.5, 0.000001),
    );
    await tester.pumpWidget(const SizedBox.shrink());
  });
}
