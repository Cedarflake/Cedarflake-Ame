import "dart:async";

import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_time_navigation.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_timeline_projection.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final geometryDelay in [0, 3]) {
    testWidgets(
      "publication aligns a changed four-column date row after $geometryDelay frames",
      (tester) async {
        final scroll = ScrollController();
        addTearDown(scroll.dispose);
        final key = GlobalKey<_PublicationHarnessState>();
        await tester.pumpWidget(
          MaterialApp(
            home: Scaffold(
              body: SizedBox(
                height: 300,
                child: _PublicationHarness(
                  key: key,
                  scroll: scroll,
                  columns: 4,
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final projection = LibraryTimelineProjection(
          timeline: key.currentState!.timeline,
          useAspectRatioWeight: false,
        );
        final value = 1 - projection.valueForGlobalItemOffset(80.25);
        final slider = tester.widget<Slider>(
          find.byKey(const Key("timeline-slider")),
        );
        slider.onChangeStart!(slider.value);
        slider.onChanged!(value);
        slider.onChangeEnd!(value);
        await tester.pump();
        await tester.pump();
        expect(key.currentState!.requests, [("2012-03", 10)]);
        expect(scroll.offset, closeTo(1680, 0.01));

        key.currentState!.publish("resolved", delayGeometry: geometryDelay > 0);
        for (var frame = 0; frame < geometryDelay; frame++) {
          await tester.pump(const Duration(milliseconds: 16));
        }
        key.currentState!.releaseGeometry();
        for (var frame = 0; frame < 10; frame++) {
          await tester.pump(const Duration(milliseconds: 16));
        }
        await tester.pumpAndSettle();
        expect(scroll.offset, closeTo(560, 0.01));
        expect(find.text("photo-28"), findsOneWidget);
        expect(find.text("photo-30"), findsOneWidget);
        expect(key.currentState!.requests, hasLength(1));
        expect(tester.takeException(), isNull);
      },
    );
  }

  for (final variant in [
    "resolved",
    "missing receipt",
    "changed query",
    "changed layout",
  ]) {
    testWidgets("date publication alignment: $variant", (tester) async {
      final scroll = ScrollController();
      addTearDown(scroll.dispose);
      final key = GlobalKey<_PublicationHarnessState>();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SizedBox(
              height: 300,
              child: _PublicationHarness(key: key, scroll: scroll),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final projection = LibraryTimelineProjection(
        timeline: key.currentState!.timeline,
        useAspectRatioWeight: false,
      );
      final value = 1 - projection.valueForGlobalItemOffset(80.25);
      final slider = tester.widget<Slider>(
        find.byKey(const Key("timeline-slider")),
      );
      slider.onChangeStart!(slider.value);
      slider.onChanged!(value);
      slider.onChangeEnd!(value);
      await tester.pump();
      await tester.pump();
      expect(key.currentState!.requests, [("2012-03", 10)]);
      expect(scroll.offset, closeTo(3200, 0.01));

      key.currentState!.publish(variant);
      for (var frame = 0; frame < 10; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
      }
      await tester.pumpAndSettle();
      if (variant == "resolved") {
        expect(scroll.offset, closeTo(1200, 0.01));
      } else {
        expect(scroll.offset, isNot(closeTo(1200, 0.01)));
      }
      expect(
        key.currentState!.requests,
        hasLength(1),
        reason: "no wheel, repeated input or compensating seek is required",
      );
      expect(tester.takeException(), isNull);
    });
  }
}

class _PublicationHarness extends StatefulWidget {
  const _PublicationHarness({
    required this.scroll,
    this.columns = 1,
    super.key,
  });
  final ScrollController scroll;
  final int columns;

  @override
  State<_PublicationHarness> createState() => _PublicationHarnessState();
}

class _PublicationHarnessState extends State<_PublicationHarness> {
  final completion = Completer<bool>();
  final requests = <(String?, int)>[];
  var published = false;
  var queryId = "query-1";
  var shape = GalleryLayoutShape.square;
  LibraryTimeAnchor? resolution;
  bool geometryReady = true;
  double get rowHeight => widget.columns == 1 ? 40 : 80;

  List<List<int>> get rows {
    final List<int> groups;
    if (widget.columns == 1) {
      groups = [timeline.totalItems];
    } else if (published) {
      groups = [20, 8, 22];
    } else {
      groups = [70, 10, 20];
    }
    final result = <List<int>>[];
    var preceding = 0;
    for (final count in groups) {
      for (var start = 0; start < count; start += widget.columns) {
        result.add([
          for (
            var item = start;
            item < count && item < start + widget.columns;
            item++
          )
            preceding + item,
        ]);
      }
      preceding += count;
    }
    return result;
  }

  int get windowStart {
    if (!published) {
      return 0;
    }
    return widget.columns == 1 ? 30 : 20;
  }

  LibraryTimeline get timeline => LibraryTimeline(
    revision: published ? BigInt.two : BigInt.one,
    queryId: queryId,
    totalItems: published ? 50 : 100,
    buckets: [
      LibraryTimeBucket(
        monthKey: "2026-09",
        itemCount: published ? 20 : 70,
        aspectRatioSum: published ? 20 : 70,
      ),
      const LibraryTimeBucket(
        monthKey: "2012-03",
        itemCount: 30,
        aspectRatioSum: 30,
      ),
    ],
  );

  void publish(String variant, {bool delayGeometry = false}) {
    setState(() {
      published = true;
      geometryReady = !delayGeometry;
      if (variant == "changed query") {
        queryId = "query-2";
      }
      if (variant == "changed layout") {
        shape = GalleryLayoutShape.equalHeight;
      }
      if (variant != "missing receipt") {
        resolution = LibraryTimeAnchor(
          revision: BigInt.two,
          queryId: queryId,
          monthKey: "2012-03",
          itemOffset: 10,
        );
      }
    });
    completion.complete(true);
  }

  void releaseGeometry() => setState(() => geometryReady = true);

  @override
  Widget build(BuildContext context) {
    final galleryRows = rows;
    final metrics = LibraryGalleryLayoutMetrics(
      contentExtent: galleryRows.length * rowHeight,
      photoRowHeight: rowHeight,
      dateAnchors: const [
        LibraryGalleryDateAnchor(
          id: "2026-09-01",
          label: "2026年9月1日",
          scrollOffset: 0,
          year: 2026,
          isUnknown: false,
        ),
      ],
      locationOffsets: const {},
      itemOffsets: [
        for (var row = 0; row < galleryRows.length; row++)
          for (final _ in galleryRows[row]) row * rowHeight,
      ],
      isQueryWide: true,
    );
    return Row(
      children: [
        Expanded(
          child: ListView.builder(
            controller: widget.scroll,
            itemExtent: rowHeight,
            itemCount: galleryRows.length,
            itemBuilder: (_, index) => Row(
              children: [
                for (final item in galleryRows[index])
                  Expanded(child: Text("photo-$item")),
              ],
            ),
          ),
        ),
        LibraryTimeNavigation(
          isLoading: false,
          scrollController: widget.scroll,
          layoutMetrics: geometryReady ? metrics : null,
          timeline: timeline,
          layoutShape: shape,
          resolvedTimeAnchor: resolution,
          windowStartItemOffset: windowStart,
          loadedItemCount: published ? 20 : 6,
          onSeek: (bucket, offset) {
            requests.add((bucket.monthKey, offset));
            return completion.future;
          },
        ),
      ],
    );
  }
}
