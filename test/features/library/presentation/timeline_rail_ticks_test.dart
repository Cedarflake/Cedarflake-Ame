import "package:cedarflake_ame/features/library/presentation/widgets/timeline_rail_ticks.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final height in [80.0, 200.0, 600.0, 4000.0]) {
    testWidgets("positional ticks stay spaced and bounded at height $height", (
      tester,
    ) async {
      await tester.binding.setSurfaceSize(Size(100, height));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      await tester.pumpWidget(
        const MaterialApp(
          home: Align(
            alignment: Alignment.centerRight,
            child: SizedBox(
              width: TimelineRailMarker.extent,
              child: TimelineRailTicks(endpointInset: 12),
            ),
          ),
        ),
      );
      final ticks = find.byType(TimelineRailMarker);
      expect(ticks.evaluate().length, inInclusiveRange(2, 21));
      final centers = [
        for (var index = 0; index < ticks.evaluate().length; index++)
          tester.getCenter(ticks.at(index)),
      ];
      expect(centers.first.dy, 12);
      expect(centers.last.dy, closeTo(height - 12, 0.001));
      for (var index = 1; index < centers.length; index++) {
        expect(centers[index].dx, centers.first.dx);
        expect(
          centers[index].dy - centers[index - 1].dy,
          greaterThanOrEqualTo(10),
        );
      }
      expect(find.byType(Text), findsNothing);
      expect(tester.takeException(), isNull);
    });
  }
}
