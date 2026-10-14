import "package:cedarflake_ame/features/library/presentation/widgets/timeline_rail_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/timeline_rail_model.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("year text does not reserve empty space on the marker axis", () {
    final layout = _layout([
      _anchor("year", 0, isYearStart: true),
      _anchor("overlapping-dot", 0.01),
      _anchor("beside-text", 0.04),
      _anchor("middle", 0.5),
    ]);
    expect(layout.labelIds, {"year"});
    expect(layout.markerIds, {"year", "beside-text", "middle"});
  });

  test("hides colliding years without moving surviving anchors", () {
    final layout = _layout([
      _anchor("first", 0, isYearStart: true),
      _anchor("crowded", 0.01, isYearStart: true),
      _anchor("middle", 0.5, isYearStart: true),
      _anchor("last", 1, isYearStart: true),
    ]);
    expect(layout.labelIds, {"first", "middle", "last"});
    expect(layout.markerIds, {"first", "middle", "last"});
  });

  test("a rail shorter than its insets has no visible annotations", () {
    final layout = _layout([
      _anchor("first", 0, isYearStart: true),
    ], railExtent: 8);
    expect(layout.labelIds, isEmpty);
    expect(layout.markerIds, isEmpty);
  });
}

TimelineRailLayout _layout(
  List<TimelineRailAnchor> anchors, {
  double railExtent = 300,
}) => TimelineRailLayout(
  anchors: anchors,
  railExtent: railExtent,
  endpointInset: 12,
  labelExtent: 24,
  markerExtent: 4,
  markerMinimumGap: 6,
);

TimelineRailAnchor _anchor(
  String id,
  double value, {
  bool isYearStart = false,
}) => TimelineRailAnchor(
  bucket: TimelineRailBucket(id: id, label: id, contentExtent: 1, year: 2026),
  value: value,
  isYearStart: isYearStart,
);
