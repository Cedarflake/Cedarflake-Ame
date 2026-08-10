import "package:cedarflake_ame/features/library/presentation/widgets/annotated_time_rail.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("positions date anchors by actual linear scroll offsets", () {
    const buckets = [
      TimelineRailBucket(
        id: "head",
        label: "2026年1月1日",
        contentExtent: 1,
        scrollOffset: 0,
        year: 2026,
      ),
      TimelineRailBucket(
        id: "middle",
        label: "2025年1月1日",
        contentExtent: 9998,
        scrollOffset: 1,
        year: 2025,
      ),
      TimelineRailBucket(
        id: "tail",
        label: "2024年1月1日",
        contentExtent: 1,
        scrollOffset: 9999,
        year: 2024,
      ),
    ];

    expect(
      timelineRailValueForBucket(buckets, "head", maximumScrollOffset: 10000),
      0,
    );
    expect(
      timelineRailValueForBucket(buckets, "middle", maximumScrollOffset: 10000),
      closeTo(0.0001, 0.000001),
    );
    expect(
      timelineRailValueForBucket(buckets, "tail", maximumScrollOffset: 10000),
      closeTo(0.9999, 0.000001),
    );
  });

  testWidgets("shows the linearly addressed date on pointer hover", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.centerRight,
            child: AnnotatedTimeRail(
              value: 0,
              maximumScrollOffset: 1000,
              buckets: const [
                TimelineRailBucket(
                  id: "head",
                  label: "2026年5月12日",
                  contentExtent: 600,
                  scrollOffset: 0,
                  year: 2026,
                ),
                TimelineRailBucket(
                  id: "tail",
                  label: "2026年4月10日",
                  contentExtent: 400,
                  scrollOffset: 600,
                  year: 2026,
                ),
              ],
              onChanged: (_) {},
            ),
          ),
        ),
      ),
    );

    final hoverRegion = tester.getRect(
      find.byKey(const Key("timeline-hover-region")),
    );
    final listener = tester.widget<Listener>(
      find.byKey(const Key("timeline-hover-region")),
    );
    listener.onPointerHover?.call(
      PointerHoverEvent(
        position: Offset(24, hoverRegion.height * 0.75),
        kind: PointerDeviceKind.mouse,
      ),
    );
    await tester.pump();
    expect(find.byKey(const Key("timeline-hover-label")), findsNothing);

    listener.onPointerHover?.call(
      PointerHoverEvent(
        position: Offset(56, hoverRegion.height * 0.75),
        kind: PointerDeviceKind.mouse,
      ),
    );
    await tester.pump();

    expect(
      tester.widget<Text>(find.byKey(const Key("timeline-hover-label"))).data,
      "2026年4月10日",
    );
  });

  testWidgets("shows one drag label and suppresses the hover preview", (
    tester,
  ) async {
    var value = 0.0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.centerRight,
            child: StatefulBuilder(
              builder: (context, setState) => AnnotatedTimeRail(
                value: value,
                maximumScrollOffset: 1000,
                buckets: const [
                  TimelineRailBucket(
                    id: "head",
                    label: "2026年5月12日",
                    contentExtent: 600,
                    scrollOffset: 0,
                    year: 2026,
                  ),
                  TimelineRailBucket(
                    id: "tail",
                    label: "2026年4月10日",
                    contentExtent: 400,
                    scrollOffset: 600,
                    year: 2026,
                  ),
                ],
                onChanged: (nextValue) => setState(() => value = nextValue),
              ),
            ),
          ),
        ),
      ),
    );

    final hoverRegion = tester.getRect(
      find.byKey(const Key("timeline-hover-region")),
    );
    tester
        .widget<Listener>(find.byKey(const Key("timeline-hover-region")))
        .onPointerHover
        ?.call(
          PointerHoverEvent(
            position: Offset(56, hoverRegion.height * 0.75),
            kind: PointerDeviceKind.mouse,
          ),
        );
    await tester.pump();
    expect(find.byKey(const Key("timeline-hover-line")), findsOneWidget);

    final slider = tester.widget<Slider>(
      find.byKey(const Key("timeline-slider")),
    );
    slider.onChangeStart?.call(slider.value);
    slider.onChanged?.call(0.25);
    await tester.pump();

    expect(find.byKey(const Key("timeline-hover-line")), findsNothing);
    expect(find.byKey(const Key("timeline-hover-label")), findsNothing);
    expect(
      tester.widget<Text>(find.byKey(const Key("timeline-drag-label"))).data,
      "2026年4月10日",
    );

    tester
        .widget<Slider>(find.byKey(const Key("timeline-slider")))
        .onChangeEnd
        ?.call(0.25);
    await tester.pump();

    expect(find.byKey(const Key("timeline-drag-label")), findsNothing);
    expect(find.byKey(const Key("timeline-hover-line")), findsNothing);
  });

  testWidgets("separates colliding year labels when the rail has room", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.centerRight,
            child: SizedBox(
              height: 300,
              child: AnnotatedTimeRail(
                value: 0,
                maximumScrollOffset: 1000,
                buckets: const [
                  TimelineRailBucket(
                    id: "year-2026",
                    label: "2026年1月1日",
                    contentExtent: 10,
                    scrollOffset: 0,
                    year: 2026,
                  ),
                  TimelineRailBucket(
                    id: "year-2025",
                    label: "2025年1月1日",
                    contentExtent: 990,
                    scrollOffset: 10,
                    year: 2025,
                  ),
                  TimelineRailBucket(
                    id: "year-2024",
                    label: "2024年1月1日",
                    contentExtent: 1,
                    scrollOffset: 1000,
                    year: 2024,
                  ),
                ],
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );

    expect(find.byKey(const ValueKey("time-label-year-2026")), findsOneWidget);
    expect(find.byKey(const ValueKey("time-label-year-2025")), findsOneWidget);
    expect(find.byKey(const ValueKey("time-label-year-2024")), findsOneWidget);
    final first = tester.getRect(
      find.byKey(const ValueKey("time-label-year-2026")),
    );
    final middle = tester.getRect(
      find.byKey(const ValueKey("time-label-year-2025")),
    );
    final last = tester.getRect(
      find.byKey(const ValueKey("time-label-year-2024")),
    );
    expect(middle.top - first.bottom, greaterThanOrEqualTo(16));
    expect(last.top - middle.bottom, greaterThanOrEqualTo(16));

    final currentLine = tester.getRect(
      find.byKey(const Key("timeline-current-line")),
    );
    expect(currentLine.center.dy, closeTo(first.center.dy, 0.001));

    final middleMarker = tester.getRect(
      find.byKey(const ValueKey("time-marker-year-2025")),
    );
    expect(middleMarker.center.dy, closeTo(middle.center.dy, 0.001));

    final hoverRegion = tester.getRect(
      find.byKey(const Key("timeline-hover-region")),
    );
    final listener = tester.widget<Listener>(
      find.byKey(const Key("timeline-hover-region")),
    );
    listener.onPointerHover?.call(
      PointerHoverEvent(
        position: Offset(56, middleMarker.center.dy - hoverRegion.top),
        kind: PointerDeviceKind.mouse,
      ),
    );
    await tester.pump();

    expect(
      tester.widget<Text>(find.byKey(const Key("timeline-hover-label"))).data,
      "2025年1月1日",
    );
  });

  testWidgets("thins crowded month markers around persistent year labels", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.centerRight,
            child: SizedBox(
              height: 300,
              child: AnnotatedTimeRail(
                value: 0,
                maximumScrollOffset: 1000,
                buckets: const [
                  TimelineRailBucket(
                    id: "year",
                    label: "2026年1月1日",
                    contentExtent: 1,
                    scrollOffset: 0,
                    year: 2026,
                  ),
                  TimelineRailBucket(
                    id: "crowded-month",
                    label: "2026年2月1日",
                    contentExtent: 499,
                    scrollOffset: 1,
                    year: 2026,
                  ),
                  TimelineRailBucket(
                    id: "separated-month",
                    label: "2026年3月1日",
                    contentExtent: 500,
                    scrollOffset: 500,
                    year: 2026,
                  ),
                ],
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );

    expect(
      find.byKey(const ValueKey("time-marker-crowded-month")),
      findsNothing,
    );
    expect(
      find.byKey(const ValueKey("time-marker-separated-month")),
      findsOneWidget,
    );
  });

  testWidgets("does not snap Slider release to a date anchor", (tester) async {
    double? completedValue;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.centerRight,
            child: AnnotatedTimeRail(
              value: 0.4,
              maximumScrollOffset: 1000,
              buckets: const [
                TimelineRailBucket(
                  id: "head",
                  label: "2026年5月12日",
                  contentExtent: 500,
                  scrollOffset: 0,
                  year: 2026,
                ),
                TimelineRailBucket(
                  id: "tail",
                  label: "2026年4月10日",
                  contentExtent: 500,
                  scrollOffset: 500,
                  year: 2026,
                ),
              ],
              onChanged: (_) {},
              onChangeEnd: (value) => completedValue = value,
            ),
          ),
        ),
      ),
    );

    tester
        .widget<Slider>(find.byKey(const Key("timeline-slider")))
        .onChangeEnd
        ?.call(0.37);

    expect(completedValue, closeTo(0.63, 0.000001));
  });

  testWidgets("maps a tap on a displaced year marker to its logical anchor", (
    tester,
  ) async {
    double? completedValue;
    const buckets = [
      TimelineRailBucket(
        id: "year-2026",
        label: "2026年1月1日",
        contentExtent: 10,
        scrollOffset: 0,
        year: 2026,
      ),
      TimelineRailBucket(
        id: "year-2025",
        label: "2025年1月1日",
        contentExtent: 10,
        scrollOffset: 10,
        year: 2025,
      ),
      TimelineRailBucket(
        id: "year-2024",
        label: "2024年1月1日",
        contentExtent: 10,
        scrollOffset: 20,
        year: 2024,
      ),
      TimelineRailBucket(
        id: "year-2022",
        label: "2022年1月1日",
        contentExtent: 10,
        scrollOffset: 30,
        year: 2022,
      ),
      TimelineRailBucket(
        id: "year-2021",
        label: "2021年1月1日",
        contentExtent: 10,
        scrollOffset: 40,
        year: 2021,
      ),
      TimelineRailBucket(
        id: "year-2019",
        label: "2019年1月1日",
        contentExtent: 950,
        scrollOffset: 50,
        year: 2019,
      ),
    ];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.centerRight,
            child: SizedBox(
              height: 500,
              child: AnnotatedTimeRail(
                value: 0,
                maximumScrollOffset: 1000,
                buckets: buckets,
                onChanged: (_) {},
                onChangeEnd: (value) => completedValue = value,
              ),
            ),
          ),
        ),
      ),
    );

    await tester.tapAt(
      tester
          .getRect(find.byKey(const ValueKey("time-marker-year-2022")))
          .center,
    );
    await tester.pump();

    expect(
      completedValue,
      closeTo(
        timelineRailValueForBucket(
          buckets,
          "year-2022",
          maximumScrollOffset: 1000,
        ),
        0.002,
      ),
    );
  });

  testWidgets("uses a passive current line without date cluster menus", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.centerRight,
            child: SizedBox(
              height: 240,
              child: AnnotatedTimeRail(
                value: 0.5,
                maximumScrollOffset: 1000,
                buckets: const [
                  TimelineRailBucket(
                    id: "a",
                    label: "2026年5月12日",
                    contentExtent: 5,
                    scrollOffset: 500,
                    year: 2026,
                  ),
                  TimelineRailBucket(
                    id: "b",
                    label: "2026年5月7日",
                    contentExtent: 5,
                    scrollOffset: 505,
                    year: 2026,
                  ),
                  TimelineRailBucket(
                    id: "c",
                    label: "2026年4月27日",
                    contentExtent: 490,
                    scrollOffset: 510,
                    year: 2026,
                  ),
                ],
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );

    expect(find.byKey(const Key("timeline-current-line")), findsOneWidget);
    expect(find.byKey(const ValueKey("time-marker-a")), findsOneWidget);
    expect(find.byType(MenuAnchor), findsNothing);
    expect(find.byType(MenuItemButton), findsNothing);
  });

  testWidgets("keeps the standard annotation point for the unknown bucket", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Align(
            alignment: Alignment.centerRight,
            child: SizedBox(
              height: 240,
              child: AnnotatedTimeRail(
                value: 0,
                maximumScrollOffset: 1000,
                buckets: const [
                  TimelineRailBucket(
                    id: "unknown",
                    label: "拍摄日期未知",
                    contentExtent: 1000,
                    scrollOffset: 0,
                    year: null,
                    isUnknown: true,
                  ),
                ],
                onChanged: (_) {},
              ),
            ),
          ),
        ),
      ),
    );

    final markerFinder = find.byKey(const ValueKey("time-marker-unknown"));
    final currentLine = tester.getRect(
      find.byKey(const Key("timeline-current-line")),
    );
    final marker = tester.getRect(markerFinder);
    final markerBox = tester.widget<DecoratedBox>(
      find.descendant(of: markerFinder, matching: find.byType(DecoratedBox)),
    );
    final markerDecoration = markerBox.decoration as BoxDecoration;

    expect(marker.center.dy, closeTo(currentLine.center.dy, 0.001));
    expect(
      markerDecoration.color,
      Theme.of(tester.element(markerFinder)).colorScheme.outline,
    );
  });
}
