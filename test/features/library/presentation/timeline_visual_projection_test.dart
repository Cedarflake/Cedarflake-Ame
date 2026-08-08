import "package:cedarflake_ame/features/library/presentation/widgets/timeline_visual_projection.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("maps logical and visual positions through the same segments", () {
    final projection = TimelineVisualProjection(const [
      TimelineVisualMappingPoint(logicalValue: 0.1, visualValue: 0.2),
      TimelineVisualMappingPoint(logicalValue: 0.2, visualValue: 0.4),
    ]);

    expect(projection.toVisual(0.1), closeTo(0.2, 0.000001));
    expect(projection.toVisual(0.15), closeTo(0.3, 0.000001));
    expect(projection.toLogical(0.3), closeTo(0.15, 0.000001));
  });

  test("round trips positions across displaced annotation anchors", () {
    final projection = TimelineVisualProjection(const [
      TimelineVisualMappingPoint(logicalValue: 0.01, visualValue: 0.08),
      TimelineVisualMappingPoint(logicalValue: 0.02, visualValue: 0.16),
      TimelineVisualMappingPoint(logicalValue: 0.8, visualValue: 0.82),
    ]);

    for (final logicalValue in [0.0, 0.005, 0.01, 0.015, 0.1, 0.8, 1.0]) {
      expect(
        projection.toLogical(projection.toVisual(logicalValue)),
        closeTo(logicalValue, 0.000001),
      );
    }
  });
}
