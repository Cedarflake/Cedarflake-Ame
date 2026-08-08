import "package:cedarflake_ame/features/library/presentation/widgets/timeline_annotation_visibility.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("hides colliding annotations while preserving boundary labels", () {
    const candidates = [
      TimelineAnnotationCandidate(id: "first", value: 0, extent: 24),
      TimelineAnnotationCandidate(id: "near-first", value: 0.01, extent: 24),
      TimelineAnnotationCandidate(id: "last", value: 1, extent: 24),
    ];

    final visible = visibleTimelineAnnotationIds(
      candidates,
      railExtent: 300,
      startInset: 12,
      endInset: 12,
    );

    expect(visible, {"first", "last"});
  });

  test("keeps the final annotation when only the endpoints collide", () {
    const candidates = [
      TimelineAnnotationCandidate(id: "first", value: 0, extent: 24),
      TimelineAnnotationCandidate(id: "last", value: 0.01, extent: 24),
    ];

    final visible = visibleTimelineAnnotationIds(
      candidates,
      railExtent: 300,
      startInset: 12,
      endInset: 12,
    );

    expect(visible, {"last"});
  });

  test("does not redistribute annotation positions", () {
    const candidates = [
      TimelineAnnotationCandidate(id: "head", value: 0, extent: 4),
      TimelineAnnotationCandidate(id: "middle", value: 0.5, extent: 4),
      TimelineAnnotationCandidate(id: "tail", value: 1, extent: 4),
    ];

    final visible = visibleTimelineAnnotationIds(
      candidates,
      railExtent: 1000,
      startInset: 12,
      endInset: 12,
    );

    expect(visible, {"head", "middle", "tail"});
  });

  test("hides annotations that do not leave the minimum visual gap", () {
    const candidates = [
      TimelineAnnotationCandidate(id: "upper", value: 0.2, extent: 4),
      TimelineAnnotationCandidate(id: "lower", value: 0.3, extent: 4),
    ];

    final visible = visibleTimelineAnnotationIds(
      candidates,
      railExtent: 100,
      startInset: 12,
      endInset: 12,
    );

    expect(visible, {"lower"});
  });

  test("keeps sparse labels and separates their display positions", () {
    const candidates = [
      TimelineAnnotationCandidate(id: "year", value: 0, extent: 24),
      TimelineAnnotationCandidate(id: "unknown", value: 0.05, extent: 24),
    ];

    final placements = layoutTimelineAnnotations(
      candidates,
      railExtent: 300,
      startInset: 12,
      endInset: 12,
    );

    expect(placements.map((placement) => placement.id), ["year", "unknown"]);
    expect(
      placements[1].center - placements[0].center,
      greaterThanOrEqualTo(32),
    );
  });
}
