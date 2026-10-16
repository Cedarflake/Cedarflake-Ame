import "package:cedarflake_ame/features/library/presentation/widgets/annotated_time_rail.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final change in ["anchors", "range", "height"]) {
    for (final reduced in [false, true]) {
      testWidgets("rail moves after $change (reduced=$reduced)", (
        tester,
      ) async {
        await tester.pumpWidget(_host(reduced: reduced));
        final marker = find.byKey(const ValueKey("time-marker-middle"));
        final label = find.byKey(const ValueKey("time-label-middle"));
        final line = find.byKey(const Key("timeline-current-line"));
        final before = [marker, label, line].map(tester.getCenter).toList();
        await tester.pumpWidget(
          _host(
            reduced: reduced,
            height: change == "height" ? 500 : 600,
            maximum: change == "range" ? 2000 : 1000,
            middle: change == "anchors" ? 600 : 300,
            value: 0.6,
          ),
        );
        final immediate = [marker, label, line].map(tester.getCenter).toList();
        if (!reduced) expect(immediate, before);
        await tester.pump(const Duration(milliseconds: 90));
        final halfway = [marker, label, line].map(tester.getCenter).toList();
        await tester.pump(const Duration(milliseconds: 90));
        final after = [marker, label, line].map(tester.getCenter).toList();
        for (var index = 0; index < before.length; index++) {
          expect(after[index].dx, before[index].dx);
          expect(after[index].dy, isNot(before[index].dy));
          if (reduced) {
            expect(immediate[index], after[index]);
          } else {
            expect(
              halfway[index].dy,
              inExclusiveRange(
                before[index].dy < after[index].dy
                    ? before[index].dy
                    : after[index].dy,
                before[index].dy > after[index].dy
                    ? before[index].dy
                    : after[index].dy,
              ),
            );
          }
        }
        expect(tester.takeException(), isNull);
      });
    }
  }

  testWidgets("scroll updates interrupt line motion immediately", (
    tester,
  ) async {
    await tester.pumpWidget(_host());
    await tester.pumpWidget(_host(middle: 600, value: 0.6));
    await tester.pump(const Duration(milliseconds: 40));
    await tester.pumpWidget(_host(middle: 600, value: 0.8));
    final rail = tester.getRect(find.byKey(const Key("timeline-hover-region")));
    final line = find.byKey(const Key("timeline-current-line"));
    expect(
      tester.getCenter(line).dy,
      closeTo(rail.top + 12 + (rail.height - 24) * 0.8, 0.001),
    );
    final immediate = tester.getRect(line);
    await tester.pumpAndSettle();
    expect(tester.getRect(line), immediate);
  });

  testWidgets(
    "reversal starts at the displayed position and reduced motion snaps",
    (tester) async {
      final marker = find.byKey(const ValueKey("time-marker-middle"));
      await tester.pumpWidget(_host());
      final original = tester.getRect(marker);
      await tester.pumpWidget(_host(middle: 600));
      await tester.pump(const Duration(milliseconds: 60));
      final midway = tester.getRect(marker);
      await tester.pumpWidget(_host());
      expect(tester.getRect(marker), midway);
      await tester.pump(const Duration(milliseconds: 30));
      await tester.pumpWidget(_host(reduced: true));
      expect(tester.getRect(marker), original);
      expect(tester.takeException(), isNull);
    },
  );
}

Widget _host({
  bool reduced = false,
  double height = 600,
  double maximum = 1000,
  double middle = 300,
  double value = 0.3,
}) => MaterialApp(
  home: MediaQuery(
    data: MediaQueryData(disableAnimations: reduced),
    child: Scaffold(
      body: Align(
        alignment: Alignment.topRight,
        child: SizedBox(
          height: height,
          child: AnnotatedTimeRail(
            value: value,
            maximumScrollOffset: maximum,
            buckets: [
              const TimelineRailBucket(
                id: "head",
                label: "2026",
                year: 2026,
                contentExtent: 300,
                scrollOffset: 0,
              ),
              TimelineRailBucket(
                id: "middle",
                label: "2025",
                year: 2025,
                contentExtent: 600,
                scrollOffset: middle,
              ),
              const TimelineRailBucket(
                id: "tail",
                label: "2024",
                year: 2024,
                contentExtent: 100,
                scrollOffset: 900,
              ),
            ],
            onChanged: (_) {},
          ),
        ),
      ),
    ),
  ),
);
