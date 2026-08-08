import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_time_navigation.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("maps rail input directly to the gallery controller", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final scrollController = ScrollController();
    addTearDown(scrollController.dispose);
    final metrics = LibraryGalleryLayoutMetrics(
      contentExtent: 1200,
      photoRowHeight: 100,
      dateAnchors: const [
        LibraryGalleryDateAnchor(
          id: "a",
          label: "2026年5月12日",
          scrollOffset: 100,
          year: 2026,
          isUnknown: false,
        ),
        LibraryGalleryDateAnchor(
          id: "b",
          label: "2026年5月7日",
          scrollOffset: 104,
          year: 2026,
          isUnknown: false,
        ),
        LibraryGalleryDateAnchor(
          id: "c",
          label: "2026年4月27日",
          scrollOffset: 108,
          year: 2026,
          isUnknown: false,
        ),
      ],
      locationOffsets: const {},
      itemOffsets: List<double>.generate(100, (index) => index * 4.0),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Row(
            children: [
              Expanded(
                child: ListView(
                  controller: scrollController,
                  children: const [SizedBox(height: 1200)],
                ),
              ),
              LibraryTimeNavigation(
                isLoading: false,
                scrollController: scrollController,
                layoutMetrics: metrics,
                timeline: LibraryTimeline(
                  revision: BigInt.one,
                  queryId: "query-1",
                  totalItems: 100,
                  buckets: const [
                    LibraryTimeBucket(
                      monthKey: "2026-05",
                      itemCount: 100,
                      aspectRatioSum: 100,
                    ),
                  ],
                ),
                layoutShape: GalleryLayoutShape.square,
                windowStartItemOffset: 0,
                loadedItemCount: 100,
                onSeek: (_, _) async => true,
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(scrollController.offset, 0);
    final slider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );
    slider.onChanged?.call(0.74);
    await tester.pump();

    expect(scrollController.offset, closeTo(104, 0.01));
    expect(find.byType(MenuAnchor), findsNothing);
  });

  testWidgets("exposes the complete bottom before later pages are loaded", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final scrollController = ScrollController();
    addTearDown(scrollController.dispose);
    LibraryTimeBucket? selectedBucket;
    int? selectedOffset;
    final metrics = LibraryGalleryLayoutMetrics(
      contentExtent: 1200,
      photoRowHeight: 100,
      dateAnchors: const [
        LibraryGalleryDateAnchor(
          id: "2026-08-05",
          label: "2026年8月5日",
          scrollOffset: 0,
          year: 2026,
          isUnknown: false,
        ),
      ],
      locationOffsets: const {},
      itemOffsets: List<double>.generate(100, (index) => index * 4.0),
    );
    final timeline = LibraryTimeline(
      revision: BigInt.one,
      queryId: "query-1",
      totalItems: 1000,
      buckets: const [
        LibraryTimeBucket(
          monthKey: "2026-08",
          itemCount: 900,
          aspectRatioSum: 900,
        ),
        LibraryTimeBucket(
          monthKey: "2022-01",
          itemCount: 100,
          aspectRatioSum: 100,
        ),
      ],
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Row(
            children: [
              Expanded(
                child: ListView(
                  controller: scrollController,
                  children: const [SizedBox(height: 1200)],
                ),
              ),
              LibraryTimeNavigation(
                isLoading: false,
                scrollController: scrollController,
                layoutMetrics: metrics,
                timeline: timeline,
                layoutShape: GalleryLayoutShape.square,
                windowStartItemOffset: 0,
                loadedItemCount: 100,
                onSeek: (bucket, itemOffset) async {
                  selectedBucket = bucket;
                  selectedOffset = itemOffset;
                  return true;
                },
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final slider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );
    slider.onChanged?.call(0);
    slider.onChangeEnd?.call(0);
    await tester.pumpAndSettle();

    expect(selectedBucket?.monthKey, "2022-01");
    expect(selectedOffset, 99);
  });

  testWidgets("uses loaded gallery row geometry for an in-window year seek", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final scrollController = ScrollController();
    addTearDown(scrollController.dispose);
    LibraryTimeBucket? selectedBucket;
    final itemOffsets = List<double>.generate(500, (index) {
      if (index <= 241) {
        return index * (900.0 / 241.0);
      }
      return 900.0 + ((index - 241) * (600.0 / 258.0));
    });
    final metrics = LibraryGalleryLayoutMetrics(
      contentExtent: 2000,
      photoRowHeight: 100,
      dateAnchors: const [
        LibraryGalleryDateAnchor(
          id: "2026-05-01",
          label: "2026年5月1日",
          scrollOffset: 0,
          year: 2026,
          isUnknown: false,
        ),
      ],
      locationOffsets: const {},
      itemOffsets: itemOffsets,
    );
    final timeline = LibraryTimeline(
      revision: BigInt.one,
      queryId: "query-1",
      totalItems: 500,
      buckets: [
        LibraryTimeBucket(monthKey: "2026-05", itemCount: 1, aspectRatioSum: 1),
        LibraryTimeBucket(
          monthKey: "2025-12",
          itemCount: 120,
          aspectRatioSum: 120,
        ),
        LibraryTimeBucket(
          monthKey: "2024-12",
          itemCount: 120,
          aspectRatioSum: 120,
        ),
        LibraryTimeBucket(
          monthKey: "2022-08",
          itemCount: 100,
          aspectRatioSum: 100,
        ),
        LibraryTimeBucket(monthKey: null, itemCount: 159, aspectRatioSum: 159),
      ],
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Row(
            children: [
              Expanded(
                child: ListView(
                  controller: scrollController,
                  children: const [SizedBox(height: 2000)],
                ),
              ),
              LibraryTimeNavigation(
                isLoading: false,
                scrollController: scrollController,
                layoutMetrics: metrics,
                timeline: timeline,
                layoutShape: GalleryLayoutShape.square,
                windowStartItemOffset: 0,
                loadedItemCount: 500,
                onSeek: (bucket, _) async {
                  selectedBucket = bucket;
                  return true;
                },
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tapAt(
      tester.getRect(find.byKey(const ValueKey("time-marker-2022-08"))).center,
    );
    await tester.pump();

    expect(scrollController.offset, closeTo(itemOffsets[241], 0.01));
    expect(selectedBucket, isNull);
  });
}
