import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_timeline_projection.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  final timeline = LibraryTimeline(
    revision: BigInt.zero,
    queryId: "query-1",
    totalItems: 1000,
    buckets: const [
      LibraryTimeBucket(
        monthKey: "2026-08",
        itemCount: 900,
        aspectRatioSum: 1800,
      ),
      LibraryTimeBucket(
        monthKey: "2022-01",
        itemCount: 100,
        aspectRatioSum: 100,
      ),
    ],
  );

  test("keeps the complete timeline range independent of loaded pages", () {
    final projection = LibraryTimelineProjection(
      timeline: timeline,
      useAspectRatioWeight: true,
    );

    expect(projection.maximumOffset, 1900);
    expect(projection.railBuckets, hasLength(2));
    expect(
      projection.valueForGlobalItemOffset(499),
      closeTo((499 / 900) * (1800 / 1900), 0.0001),
    );
    expect(projection.valueForGlobalItemOffset(1000), 1);
  });

  test("maps the bottom to the final item without walking every page", () {
    final projection = LibraryTimelineProjection(
      timeline: timeline,
      useAspectRatioWeight: true,
    );

    final target = projection.targetForValue(1);

    expect(target.bucket.monthKey, "2022-01");
    expect(target.itemOffset, 99);
    expect(target.globalItemOffset, 999);
  });

  test("stabilizes a year anchor at the adjacent bucket boundary", () {
    final projection = LibraryTimelineProjection(
      timeline: timeline,
      useAspectRatioWeight: true,
    );
    final exactBoundary = projection.valueForGlobalItemOffset(900);

    final target = projection.targetForValue(exactBoundary - 0.0000000001);

    expect(target.bucket.monthKey, "2022-01");
    expect(target.itemOffset, 0);
    expect(target.globalItemOffset, 900);
  });
}
