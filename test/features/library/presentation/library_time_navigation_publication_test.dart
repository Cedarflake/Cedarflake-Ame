import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/annotated_time_rail.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_time_navigation.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("single pointer clicks reach both rail endpoints", (
    tester,
  ) async {
    final fixture = _PublicationFixture(tester);
    await fixture.show(isQueryWide: true);
    await fixture.show(isQueryWide: true);
    for (final inset in [1.0, 12.0]) {
      fixture.controller.jumpTo(1800);
      await tester.pumpAndSettle();
      final track = tester.getRect(
        find.byKey(const Key("timeline-track-background")),
      );
      await tester.tapAt(
        Offset(track.center.dx, track.top + inset),
        kind: PointerDeviceKind.mouse,
      );
      await tester.pumpAndSettle();
      expect(fixture.controller.offset, closeTo(0, 0.01));
      await tester.tapAt(
        Offset(track.center.dx, track.bottom - inset),
        kind: PointerDeviceKind.mouse,
      );
      await tester.pumpAndSettle();
      expect(
        fixture.controller.offset,
        closeTo(fixture.controller.position.maxScrollExtent, 0.01),
      );
    }
  });

  testWidgets("replacement window estimates cannot reset the retained marker", (
    tester,
  ) async {
    final fixture = _PublicationFixture(tester);
    await fixture.show(isQueryWide: true);
    await fixture.show(isQueryWide: true);
    fixture.controller.jumpTo(1800);
    await tester.pump();
    final before = tester.widget<AnnotatedTimeRail>(
      find.byKey(const Key("library-time-rail")),
    );
    expect(before.value, greaterThan(0));

    await fixture.show(revision: 2, loadedItemCount: 0);
    final replacing = tester.widget<AnnotatedTimeRail>(
      find.byKey(const Key("library-time-rail")),
    );
    expect(replacing.value, before.value);
    expect(replacing.onChanged, isNotNull);

    await fixture.show(revision: 2, isQueryWide: true);
    await fixture.show(revision: 2, isQueryWide: true);
    expect(
      tester
          .widget<AnnotatedTimeRail>(find.byKey(const Key("library-time-rail")))
          .value,
      before.value,
    );
  });

  for (final replacement in ["missing", "estimated", "unsettled extent"]) {
    testWidgets("complete rail survives $replacement update geometry", (
      tester,
    ) async {
      final fixture = _PublicationFixture(tester);
      await fixture.show(isQueryWide: true);
      await fixture.show(isQueryWide: true);
      fixture.controller.jumpTo(1800);
      await tester.pump();
      final before = tester.widget<AnnotatedTimeRail>(
        find.byKey(const Key("library-time-rail")),
      );
      final oldSlider = tester.widget<Slider>(
        find.byKey(const Key("timeline-slider")),
      );

      await fixture.show(
        revision: 2,
        hasGeometry: replacement != "missing",
        isQueryWide: replacement != "estimated",
        metricsExtent: replacement == "unsettled extent" ? 6000 : 5000,
      );
      final during = tester.widget<AnnotatedTimeRail>(
        find.byKey(const Key("library-time-rail")),
      );
      expect(during.buckets, same(before.buckets));
      expect(during.value, before.value);
      expect(during.maximumScrollOffset, before.maximumScrollOffset);
      expect(during.onChanged, isNotNull);
      expect(during.onStep, replacement == "missing" ? isNull : isNotNull);
      oldSlider.onChanged?.call(0.1);
      oldSlider.onChangeEnd?.call(0.1);
      await tester.pump();
      expect(fixture.controller.offset, 1800);
      expect(fixture.seeks, 0);

      await fixture.show(
        revision: 2,
        isQueryWide: true,
        metricsExtent: 6000,
        contentExtent: 6000,
      );
      await fixture.show(
        revision: 2,
        isQueryWide: true,
        metricsExtent: 6000,
        contentExtent: 6000,
      );
      final after = tester.widget<AnnotatedTimeRail>(
        find.byKey(const Key("library-time-rail")),
      );
      expect(
        after.maximumScrollOffset,
        fixture.controller.position.maxScrollExtent,
      );
      expect(after.maximumScrollOffset, isNot(before.maximumScrollOffset));
      expect(after.onChanged, isNotNull);
      fixture.controller.jumpTo(2200);
      await tester.pump();
      expect(
        tester
            .widget<AnnotatedTimeRail>(
              find.byKey(const Key("library-time-rail")),
            )
            .value,
        closeTo(2200 / fixture.controller.position.maxScrollExtent, 0.000001),
      );
    });
  }

  testWidgets("initial and changed-query rails can use estimated geometry", (
    tester,
  ) async {
    final fixture = _PublicationFixture(tester);
    await fixture.show();
    expect(
      tester.widget<Slider>(find.byKey(const Key("timeline-slider"))).onChanged,
      isNotNull,
    );
    await fixture.show(isQueryWide: true);
    await fixture.show(queryId: "other-query");
    final rail = tester.widget<AnnotatedTimeRail>(
      find.byKey(const Key("library-time-rail")),
    );
    expect(
      rail.maximumScrollOffset,
      isNot(fixture.controller.position.maxScrollExtent),
    );
    expect(rail.onChanged, isNotNull);
  });

  for (final hasGeometry in [false, true]) {
    testWidgets(
      "retained ticks accept a new date seek with current geometry $hasGeometry",
      (tester) async {
        final fixture = _PublicationFixture(tester);
        await fixture.show(isQueryWide: true);
        await fixture.show(isQueryWide: true);
        final before = tester.widget<AnnotatedTimeRail>(
          find.byKey(const Key("library-time-rail")),
        );
        await fixture.show(
          revision: 2,
          hasGeometry: hasGeometry,
          loadedItemCount: 0,
          buckets: const [
            LibraryTimeBucket(
              monthKey: "2026-09",
              itemCount: 900,
              aspectRatioSum: 900,
            ),
            LibraryTimeBucket(
              monthKey: "2012-03",
              itemCount: 100,
              aspectRatioSum: 100,
            ),
          ],
        );
        final during = tester.widget<AnnotatedTimeRail>(
          find.byKey(const Key("library-time-rail")),
        );
        expect(during.buckets, same(before.buckets));
        final bounds = tester.getRect(find.byKey(const Key("timeline-slider")));
        final oldLine = tester.getRect(
          find.byKey(const Key("timeline-current-line")),
        );
        final gesture = await tester.startGesture(
          Offset(bounds.center.dx, bounds.top + bounds.height * 0.8),
        );
        await tester.pump();
        expect(
          tester.getRect(find.byKey(const Key("timeline-current-line"))).top,
          greaterThan(oldLine.top),
        );
        await gesture.up();
        await tester.pump();
        await tester.pump(const Duration(milliseconds: 200));
        expect(fixture.seeks, greaterThan(0));
        expect(fixture.lastMonth, "2012-03");
        expect(fixture.lastItemOffset, inInclusiveRange(0, 99));
      },
    );
  }

  testWidgets("same-revision fallback retires an active pointer", (
    tester,
  ) async {
    final fixture = _PublicationFixture(tester);
    await fixture.show(isQueryWide: true);
    await fixture.show(isQueryWide: true);
    final bounds = tester.getRect(find.byKey(const Key("timeline-slider")));
    final gesture = await tester.startGesture(bounds.center);
    await tester.pump();
    await fixture.show();
    fixture.controller.jumpTo(1000);
    await fixture.show(isQueryWide: true);
    await gesture.up();
    await tester.pump(const Duration(milliseconds: 200));
    expect(fixture.controller.offset, 1000);
    expect(fixture.seeks, 0);
    expect(
      tester
          .widget<AnnotatedTimeRail>(find.byKey(const Key("library-time-rail")))
          .value,
      closeTo(1000 / fixture.controller.position.maxScrollExtent, 0.000001),
    );
    await tester.tapAt(
      Offset(bounds.center.dx, bounds.top + bounds.height * 0.3),
    );
    await tester.pump();
    expect(fixture.controller.offset, isNot(1000));
  });

  testWidgets(
    "a retired pointer cannot commit after replacement geometry arrives",
    (tester) async {
      final fixture = _PublicationFixture(tester);
      await fixture.show();
      final slider = find.byKey(const Key("timeline-slider"));
      final bounds = tester.getRect(slider);
      final gesture = await tester.startGesture(
        Offset(bounds.center.dx, bounds.top + bounds.height * 0.65),
      );
      await tester.pump();
      await fixture.show(revision: 2, hasGeometry: false);
      fixture.controller.jumpTo(1000);
      await fixture.show(revision: 2);
      await gesture.up();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 200));
      expect(
        fixture.controller.offset,
        1000,
        reason: "PointerUp belongs to the retired catalog revision",
      );
      expect(fixture.seeks, 0);
      await tester.tapAt(
        Offset(bounds.center.dx, bounds.top + bounds.height * 0.45),
      );
      await tester.pump();
      expect(
        fixture.controller.offset,
        isNot(1000),
        reason: "The replacement rail accepts a new pointer",
      );
    },
  );

  testWidgets("keeps the rail visible without granting stale geometry input", (
    tester,
  ) async {
    final fixture = _PublicationFixture(tester);
    await fixture.show();
    fixture.controller.jumpTo(1800);
    await tester.pump();
    final originalLine = tester.getRect(
      find.byKey(const Key("timeline-current-line")),
    );
    final oldSlider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );

    await fixture.show(revision: 2, hasGeometry: false);
    expect(find.byKey(const Key("library-time-rail")), findsOneWidget);
    expect(
      tester.getRect(find.byKey(const Key("timeline-current-line"))),
      originalLine,
    );
    expect(
      tester.widget<Slider>(find.byKey(const Key("timeline-slider"))).onChanged,
      isNotNull,
    );
    expect(
      tester
          .widget<IconButton>(find.byKey(const Key("timeline-next")))
          .onPressed,
      isNotNull,
    );
    oldSlider.onChanged?.call(0.1);
    oldSlider.onChangeEnd?.call(0.1);
    await tester.pump(const Duration(milliseconds: 200));
    expect(fixture.controller.offset, 1800);
    expect(fixture.seeks, 0);

    await fixture.show(revision: 2);
    final currentSlider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );
    expect(currentSlider.onChanged, isNotNull);
    currentSlider.onChanged!.call(0.3);
    await tester.pump();
    expect(fixture.controller.offset, isNot(1800));
  });

  testWidgets("layout replacement retains paint and retires old slider input", (
    tester,
  ) async {
    final fixture = _PublicationFixture(tester);
    await fixture.show();
    final marker = find.byKey(const Key("timeline-current-line"));
    final element = tester.element(marker);
    final oldSlider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );
    await fixture.show(
      hasGeometry: false,
      shape: GalleryLayoutShape.equalHeight,
    );
    expect(tester.element(marker), same(element));
    expect(
      tester.widget<Slider>(find.byKey(const Key("timeline-slider"))).onChanged,
      isNotNull,
    );
    oldSlider.onChanged?.call(0.1);
    oldSlider.onChangeEnd?.call(0.1);
    await tester.pump();
    expect(fixture.controller.offset, 0);
    expect(fixture.seeks, 0);
    await fixture.show(shape: GalleryLayoutShape.equalHeight);
    expect(tester.element(marker), same(element));
    expect(
      tester.widget<Slider>(find.byKey(const Key("timeline-slider"))).onChanged,
      isNotNull,
    );
  });

  testWidgets("query replacement retains paint and animates the new layout", (
    tester,
  ) async {
    final fixture = _PublicationFixture(tester);
    await fixture.show();
    fixture.controller.jumpTo(1800);
    await tester.pumpAndSettle();
    final marker = find.byKey(const Key("timeline-current-line"));
    final element = tester.element(marker);
    final before = tester.getCenter(marker);
    final oldSlider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );

    await fixture.show(queryId: "modified-time", hasGeometry: false);
    expect(tester.element(marker), same(element));
    expect(tester.getCenter(marker), before);
    expect(
      tester.widget<Slider>(find.byKey(const Key("timeline-slider"))).onChanged,
      isNull,
    );
    oldSlider.onChanged?.call(0.1);
    oldSlider.onChangeEnd?.call(0.1);
    await tester.pump();
    expect(fixture.controller.offset, 1800);
    expect(fixture.seeks, 0);

    await fixture.show(queryId: "modified-time", isQueryWide: true);
    expect(tester.element(marker), same(element));
    expect(tester.getCenter(marker), before);
    await tester.pump(const Duration(milliseconds: 90));
    final halfway = tester.getCenter(marker);
    await tester.pump(const Duration(milliseconds: 90));
    final after = tester.getCenter(marker);
    final currentRail = tester.widget<AnnotatedTimeRail>(
      find.byType(AnnotatedTimeRail),
    );
    expect(currentRail.maximumScrollOffset, 4400);
    expect(currentRail.value, closeTo(1800 / 4400, 0.000001));
    expect(after.dy, greaterThan(before.dy));
    expect(halfway.dy, inExclusiveRange(before.dy, after.dy));
    final currentSlider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );
    expect(currentSlider.onChanged, isNotNull);
    currentSlider.onChanged!.call(0.4);
    await tester.pump();
    expect(fixture.controller.offset, isNot(1800));
  });

  for (final change in ["empty", "controller"]) {
    testWidgets("does not retain a rail across a $change change", (
      tester,
    ) async {
      final fixture = _PublicationFixture(tester);
      await fixture.show();
      expect(find.byKey(const Key("library-time-rail")), findsOneWidget);
      if (change == "controller") {
        fixture.controller = ScrollController();
        addTearDown(fixture.controller.dispose);
      }
      await fixture.show(
        revision: 2,
        hasGeometry: false,
        isEmpty: change == "empty",
      );
      expect(find.byKey(const Key("library-time-rail")), findsNothing);
    });
  }
}

class _PublicationFixture {
  _PublicationFixture(this.tester) {
    addTearDown(controller.dispose);
  }

  final WidgetTester tester;
  ScrollController controller = ScrollController();
  int seeks = 0;
  String? lastMonth;
  int? lastItemOffset;

  Future<void> show({
    int revision = 1,
    bool hasGeometry = true,
    bool isEmpty = false,
    bool isQueryWide = false,
    double contentExtent = 5000,
    double metricsExtent = 5000,
    String queryId = "query-1",
    GalleryLayoutShape shape = GalleryLayoutShape.square,
    int loadedItemCount = 100,
    List<LibraryTimeBucket>? buckets,
  }) async {
    final currentBuckets =
        buckets ??
        const [
          LibraryTimeBucket(
            monthKey: "2026-09",
            itemCount: 50,
            aspectRatioSum: 50,
          ),
          LibraryTimeBucket(
            monthKey: "2012-03",
            itemCount: 50,
            aspectRatioSum: 50,
          ),
        ];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Row(
            children: [
              Expanded(
                child: ListView(
                  controller: controller,
                  padding: EdgeInsets.zero,
                  children: [SizedBox(height: contentExtent)],
                ),
              ),
              LibraryTimeNavigation(
                isLoading: false,
                scrollController: controller,
                layoutMetrics: hasGeometry
                    ? LibraryGalleryLayoutMetrics(
                        contentExtent: metricsExtent,
                        isQueryWide: isQueryWide,
                        photoRowHeight: 100,
                        dateAnchors: const [
                          LibraryGalleryDateAnchor(
                            id: "2026-09-01",
                            label: "2026",
                            scrollOffset: 0,
                            year: 2026,
                            isUnknown: false,
                          ),
                        ],
                        locationOffsets: const {},
                        itemOffsets: List.generate(
                          100,
                          (index) => index * 50.0,
                        ),
                      )
                    : null,
                timeline: LibraryTimeline(
                  revision: BigInt.from(revision),
                  queryId: queryId,
                  totalItems: isEmpty
                      ? 0
                      : currentBuckets.fold(
                          0,
                          (sum, bucket) => sum + bucket.itemCount,
                        ),
                  buckets: isEmpty ? const [] : currentBuckets,
                ),
                layoutShape: shape,
                windowStartItemOffset: 0,
                loadedItemCount: loadedItemCount,
                onSeek: (bucket, itemOffset) async {
                  seeks += 1;
                  lastMonth = bucket.monthKey;
                  lastItemOffset = itemOffset;
                  return true;
                },
              ),
            ],
          ),
        ),
      ),
    );
  }
}
