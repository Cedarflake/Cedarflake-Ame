import "package:cedarflake_ame/features/library/presentation/widgets/annotated_time_rail.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("positions bucket nodes by nonuniform content extent", () {
    const buckets = [
      TimelineRailBucket(
        id: "2026-08",
        label: "2026年8月",
        contentExtent: 900,
        year: 2026,
      ),
      TimelineRailBucket(
        id: "2025-01",
        label: "2025年1月",
        contentExtent: 100,
        year: 2025,
      ),
    ];

    expect(timelineRailValueForBucket(buckets, "2026-08"), 0);
    expect(timelineRailValueForBucket(buckets, "2025-01"), 1);
  });
}
