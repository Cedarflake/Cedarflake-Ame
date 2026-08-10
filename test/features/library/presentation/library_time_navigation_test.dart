import "dart:async";

import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_time_navigation.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_timeline_projection.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_virtual_gallery_geometry.dart";
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
    var scrollNotifications = 0;
    scrollController.addListener(() => scrollNotifications += 1);
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
    expect(scrollController.offset, 0);
    await tester.pump();

    expect(scrollController.offset, closeTo(104, 0.01));
    final previousNotifications = scrollNotifications;
    slider.onChanged?.call(0.4);
    slider.onChanged?.call(0.3);
    slider.onChanged?.call(0.25);
    expect(scrollNotifications, previousNotifications);
    await tester.pump();

    expect(scrollController.offset, closeTo(300, 0.01));
    expect(scrollNotifications, previousNotifications + 1);
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
    slider.onChangeStart?.call(slider.value);
    slider.onChanged?.call(0);
    await tester.pumpAndSettle();

    expect(selectedBucket, isNull);
    expect(selectedOffset, isNull);

    tester
        .widget<Slider>(find.byKey(const Key("timeline-slider")))
        .onChangeEnd
        ?.call(0);
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

  testWidgets("loads a distant equal-height target before drag release", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final scrollController = ScrollController();
    addTearDown(scrollController.dispose);
    final selectedOffsets = <int>[];
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
    const virtualGeometry = LibraryVirtualGalleryGeometry(
      totalContentExtent: 10000,
      viewportExtent: 800,
      leadingExtent: 0,
      loadedContentExtent: 1200,
      trailingExtent: 8800,
      windowStartItemOffset: 0,
      loadedItemCount: 100,
      totalItemCount: 1000,
      queryId: "query-1",
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Row(
            children: [
              Expanded(
                child: ListView(
                  controller: scrollController,
                  children: const [SizedBox(height: 10000)],
                ),
              ),
              LibraryTimeNavigation(
                isLoading: false,
                scrollController: scrollController,
                layoutMetrics: metrics,
                timeline: LibraryTimeline(
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
                ),
                layoutShape: GalleryLayoutShape.equalHeight,
                virtualGeometry: virtualGeometry,
                windowStartItemOffset: 0,
                loadedItemCount: 100,
                onSeek: (_, itemOffset) {
                  selectedOffsets.add(itemOffset);
                  return Future.value(true);
                },
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    var slider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );
    slider.onChangeStart?.call(slider.value);
    slider.onChanged?.call(0.5);
    await tester.pump();
    await tester.pump();
    expect(selectedOffsets, isNotEmpty);
    expect(scrollController.offset, 0);

    slider = tester.widget<Slider>(find.byKey(const Key("timeline-slider")));
    slider.onChanged?.call(0);
    await tester.pump(const Duration(milliseconds: 120));
    expect(scrollController.offset, 0);

    tester
        .widget<Slider>(find.byKey(const Key("timeline-slider")))
        .onChangeEnd
        ?.call(0);
    await tester.pumpAndSettle();

    expect(selectedOffsets.last, 99);
    expect(selectedOffsets.length, lessThanOrEqualTo(3));
  });

  testWidgets(
    "keeps the rail visible and aligns a published equal-height window",
    (tester) async {
      tester.view.physicalSize = const Size(1000, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      final scrollController = ScrollController();
      addTearDown(scrollController.dispose);

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: _PublishingSeekHarness(scrollController: scrollController),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(tester.getSize(find.byType(LibraryTimeNavigation)).width, 80);

      final slider = tester.widget<Slider>(
        find.byKey(const Key("timeline-slider")),
      );
      slider.onChangeStart?.call(slider.value);
      slider.onChanged?.call(0);
      slider.onChangeEnd?.call(0);

      await tester.pump();
      expect(find.byKey(const Key("library-time-rail")), findsOneWidget);
      expect(tester.getSize(find.byType(LibraryTimeNavigation)).width, 80);
      await tester.pumpAndSettle();

      expect(find.byKey(const Key("library-time-rail")), findsOneWidget);
      expect(tester.getSize(find.byType(LibraryTimeNavigation)).width, 80);
      expect(scrollController.offset, greaterThan(9000));
    },
  );

  testWidgets("waits for gallery scrolling to settle before seeking", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final scrollController = ScrollController();
    addTearDown(scrollController.dispose);
    final selectedOffsets = <int>[];
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
    const virtualGeometry = LibraryVirtualGalleryGeometry(
      totalContentExtent: 10000,
      viewportExtent: 800,
      leadingExtent: 0,
      loadedContentExtent: 1200,
      trailingExtent: 8800,
      windowStartItemOffset: 0,
      loadedItemCount: 100,
      totalItemCount: 1000,
      queryId: "query-1",
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Row(
            children: [
              Expanded(
                child: ListView(
                  controller: scrollController,
                  children: const [SizedBox(height: 10000)],
                ),
              ),
              LibraryTimeNavigation(
                isLoading: false,
                scrollController: scrollController,
                layoutMetrics: metrics,
                timeline: LibraryTimeline(
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
                ),
                layoutShape: GalleryLayoutShape.square,
                virtualGeometry: virtualGeometry,
                windowStartItemOffset: 0,
                loadedItemCount: 100,
                onSeek: (_, itemOffset) {
                  selectedOffsets.add(itemOffset);
                  return Future.value(true);
                },
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    scrollController.jumpTo(3000);
    await tester.pump(const Duration(milliseconds: 100));
    scrollController.jumpTo(5000);
    await tester.pump(const Duration(milliseconds: 179));
    expect(selectedOffsets, isEmpty);

    await tester.pump(const Duration(milliseconds: 1));
    expect(selectedOffsets, hasLength(1));
  });

  testWidgets(
    "query-wide wheel leaves visible detail loading to the gallery wall",
    (tester) async {
      tester.view.physicalSize = const Size(1000, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      final scrollController = ScrollController();
      addTearDown(scrollController.dispose);
      final selectedOffsets = <int>[];

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: _PublishingWheelHarness(
              scrollController: scrollController,
              onSeek: selectedOffsets.add,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      scrollController.jumpTo(5055);
      await tester.pump(const Duration(milliseconds: 180));
      expect(selectedOffsets, isEmpty);
      expect(scrollController.offset, closeTo(5055, 0.01));
    },
  );

  testWidgets("an obsolete seek completion cannot pull back a newer target", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final scrollController = ScrollController();
    addTearDown(scrollController.dispose);
    final requests = _ControlledSeekRequests();

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: _LatestSeekHarness(
            scrollController: scrollController,
            requests: requests,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();

    var slider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );
    slider.onChangeStart?.call(slider.value);
    slider.onChanged?.call(0.75);
    await tester.pump();
    await tester.pump();
    expect(requests.offsets, hasLength(1));

    slider = tester.widget<Slider>(find.byKey(const Key("timeline-slider")));
    slider.onChanged?.call(0.2);
    slider.onChangeEnd?.call(0.2);
    await tester.pump();
    final latestPixels = scrollController.offset;
    expect(latestPixels, greaterThan(7000));
    expect(requests.offsets, hasLength(2));

    requests.completers.first.complete(true);
    await tester.pump();
    await tester.pump();

    expect(scrollController.offset, closeTo(latestPixels, 0.01));
    expect(requests.offsets, hasLength(2));
    requests.completers.last.complete(true);
    await tester.pumpAndSettle();

    expect(scrollController.offset, closeTo(latestPixels, 0.01));
  });

  testWidgets(
    "loads a query-wide target row from its first item without a wheel seek",
    (tester) async {
      tester.view.physicalSize = const Size(1000, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);

      final scrollController = ScrollController();
      addTearDown(scrollController.dispose);
      final requestedOffsets = <int>[];
      final projection = LibraryTimelineProjection(
        timeline: _RowAlignedSeekHarness.timeline,
        useAspectRatioWeight: true,
      );
      final targetValue = projection.valueForGlobalItemOffset(8);

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: _RowAlignedSeekHarness(
              scrollController: scrollController,
              requestedOffsets: requestedOffsets,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();

      final slider = tester.widget<Slider>(
        find.byKey(const Key("timeline-slider")),
      );
      final sliderTarget = 1 - targetValue;
      slider.onChangeStart?.call(slider.value);
      slider.onChanged?.call(sliderTarget);
      slider.onChangeEnd?.call(sliderTarget);
      await tester.pump();
      await tester.pumpAndSettle();

      expect(requestedOffsets, [6]);
      expect(scrollController.offset, closeTo(240, 0.01));

      await tester.pump(const Duration(milliseconds: 200));
      expect(
        requestedOffsets,
        [6],
        reason: "the aligned row must not require a second wheel-settle seek",
      );
    },
  );
}

class _RowAlignedSeekHarness extends StatefulWidget {
  const _RowAlignedSeekHarness({
    required this.scrollController,
    required this.requestedOffsets,
  });

  static final LibraryTimeline timeline = LibraryTimeline(
    revision: BigInt.one,
    queryId: "query-1",
    totalItems: 30,
    buckets: const [
      LibraryTimeBucket(monthKey: "2026-08", itemCount: 30, aspectRatioSum: 30),
    ],
  );

  final ScrollController scrollController;
  final List<int> requestedOffsets;

  @override
  State<_RowAlignedSeekHarness> createState() => _RowAlignedSeekHarnessState();
}

class _RowAlignedSeekHarnessState extends State<_RowAlignedSeekHarness> {
  static final LibraryGalleryLayoutMetrics _metrics =
      LibraryGalleryLayoutMetrics(
        contentExtent: 1200,
        photoRowHeight: 200,
        dateAnchors: const [
          LibraryGalleryDateAnchor(
            id: "2026-08-10",
            label: "2026年8月10日",
            scrollOffset: 0,
            year: 2026,
            isUnknown: false,
          ),
        ],
        locationOffsets: const {},
        itemOffsets: [
          for (var row = 0; row < 5; row++)
            for (var item = 0; item < 6; item++) row * 240.0,
        ],
        isQueryWide: true,
      );

  var _windowStartItemOffset = 0;
  var _loadedItemCount = 6;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Expanded(
          child: ListView(
            controller: widget.scrollController,
            children: const [SizedBox(height: 1600)],
          ),
        ),
        LibraryTimeNavigation(
          isLoading: false,
          scrollController: widget.scrollController,
          layoutMetrics: _metrics,
          timeline: _RowAlignedSeekHarness.timeline,
          layoutShape: GalleryLayoutShape.equalHeight,
          windowStartItemOffset: _windowStartItemOffset,
          loadedItemCount: _loadedItemCount,
          onSeek: (_, itemOffset) async {
            widget.requestedOffsets.add(itemOffset);
            setState(() {
              _windowStartItemOffset = itemOffset;
              _loadedItemCount = 12;
            });
            return true;
          },
        ),
      ],
    );
  }
}

class _ControlledSeekRequests {
  final List<int> offsets = [];
  final List<Completer<bool>> completers = [];
}

class _LatestSeekHarness extends StatefulWidget {
  const _LatestSeekHarness({
    required this.scrollController,
    required this.requests,
  });

  final ScrollController scrollController;
  final _ControlledSeekRequests requests;

  @override
  State<_LatestSeekHarness> createState() => _LatestSeekHarnessState();
}

class _LatestSeekHarnessState extends State<_LatestSeekHarness> {
  static final LibraryTimeline _timeline = LibraryTimeline(
    revision: BigInt.one,
    queryId: "query-1",
    totalItems: 1000,
    buckets: const [
      LibraryTimeBucket(
        monthKey: "2026-08",
        itemCount: 1000,
        aspectRatioSum: 1000,
      ),
    ],
  );
  static final LibraryGalleryLayoutMetrics _metrics =
      LibraryGalleryLayoutMetrics(
        contentExtent: 10000,
        photoRowHeight: 100,
        dateAnchors: const [
          LibraryGalleryDateAnchor(
            id: "2026-08-05",
            label: "2026-08-05",
            scrollOffset: 0,
            year: 2026,
            isUnknown: false,
          ),
        ],
        locationOffsets: const {},
        itemOffsets: List<double>.generate(1000, (index) => index * 10.0),
        isQueryWide: true,
      );

  var _windowStartItemOffset = 0;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Expanded(
          child: ListView(
            controller: widget.scrollController,
            children: const [SizedBox(height: 10800)],
          ),
        ),
        LibraryTimeNavigation(
          isLoading: false,
          scrollController: widget.scrollController,
          layoutMetrics: _metrics,
          timeline: _timeline,
          layoutShape: GalleryLayoutShape.equalHeight,
          windowStartItemOffset: _windowStartItemOffset,
          loadedItemCount: 100,
          onSeek: (_, itemOffset) async {
            final completion = Completer<bool>();
            widget.requests.offsets.add(itemOffset);
            widget.requests.completers.add(completion);
            final didSeek = await completion.future;
            if (didSeek && mounted) {
              setState(() => _windowStartItemOffset = itemOffset);
            }
            return didSeek;
          },
        ),
      ],
    );
  }
}

class _PublishingWheelHarness extends StatefulWidget {
  const _PublishingWheelHarness({
    required this.scrollController,
    required this.onSeek,
  });

  final ScrollController scrollController;
  final ValueChanged<int> onSeek;

  @override
  State<_PublishingWheelHarness> createState() =>
      _PublishingWheelHarnessState();
}

class _PublishingWheelHarnessState extends State<_PublishingWheelHarness> {
  static final LibraryTimeline _timeline = LibraryTimeline(
    revision: BigInt.one,
    queryId: "query-1",
    totalItems: 1000,
    buckets: const [
      LibraryTimeBucket(
        monthKey: "2026-08",
        itemCount: 1000,
        aspectRatioSum: 1000,
      ),
    ],
  );
  static final LibraryGalleryLayoutMetrics _metrics =
      LibraryGalleryLayoutMetrics(
        contentExtent: 10000,
        photoRowHeight: 100,
        dateAnchors: const [
          LibraryGalleryDateAnchor(
            id: "2026-08-05",
            label: "2026-08-05",
            scrollOffset: 0,
            year: 2026,
            isUnknown: false,
          ),
        ],
        locationOffsets: const {},
        itemOffsets: List<double>.generate(1000, (index) => index * 10.0),
        isQueryWide: true,
      );

  var _windowStartItemOffset = 0;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Expanded(
          child: ListView(
            controller: widget.scrollController,
            children: const [SizedBox(height: 10000)],
          ),
        ),
        LibraryTimeNavigation(
          isLoading: false,
          scrollController: widget.scrollController,
          layoutMetrics: _metrics,
          timeline: _timeline,
          layoutShape: GalleryLayoutShape.equalHeight,
          windowStartItemOffset: _windowStartItemOffset,
          loadedItemCount: 100,
          onSeek: (_, itemOffset) async {
            widget.onSeek(itemOffset);
            setState(() => _windowStartItemOffset = itemOffset);
            return true;
          },
        ),
      ],
    );
  }
}

class _PublishingSeekHarness extends StatefulWidget {
  const _PublishingSeekHarness({required this.scrollController});

  final ScrollController scrollController;

  @override
  State<_PublishingSeekHarness> createState() => _PublishingSeekHarnessState();
}

class _PublishingSeekHarnessState extends State<_PublishingSeekHarness> {
  static final LibraryTimeline _timeline = LibraryTimeline(
    revision: BigInt.one,
    queryId: "query-1",
    totalItems: 1000,
    buckets: const [
      LibraryTimeBucket(
        monthKey: "2026-08",
        itemCount: 1000,
        aspectRatioSum: 1000,
      ),
    ],
  );
  static final LibraryGalleryLayoutMetrics _metrics =
      LibraryGalleryLayoutMetrics(
        contentExtent: 800,
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
        itemOffsets: const [0],
      );

  var _windowStartItemOffset = 0;
  var _isPublishingLayout = false;

  @override
  Widget build(BuildContext context) {
    final leadingExtent = _windowStartItemOffset / 999 * 9200;
    final geometry = LibraryVirtualGalleryGeometry(
      totalContentExtent: 10000,
      viewportExtent: 800,
      leadingExtent: leadingExtent,
      loadedContentExtent: 800,
      trailingExtent: 10000 - leadingExtent - 800,
      windowStartItemOffset: _windowStartItemOffset,
      loadedItemCount: 1,
      totalItemCount: 1000,
      queryId: "query-1",
    );
    return Row(
      children: [
        Expanded(
          child: ListView(
            controller: widget.scrollController,
            children: const [SizedBox(height: 10000)],
          ),
        ),
        LibraryTimeNavigation(
          isLoading: false,
          scrollController: widget.scrollController,
          layoutMetrics: _isPublishingLayout ? null : _metrics,
          timeline: _timeline,
          layoutShape: GalleryLayoutShape.equalHeight,
          virtualGeometry: _isPublishingLayout ? null : geometry,
          windowStartItemOffset: _windowStartItemOffset,
          loadedItemCount: 1,
          onSeek: (_, itemOffset) async {
            setState(() {
              _windowStartItemOffset = itemOffset;
              _isPublishingLayout = true;
            });
            WidgetsBinding.instance.addPostFrameCallback((_) {
              if (mounted) {
                setState(() => _isPublishingLayout = false);
              }
            });
            return true;
          },
        ),
      ],
    );
  }
}
