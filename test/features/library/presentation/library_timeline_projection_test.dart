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

  test("maps an exact global item to its owning time bucket", () {
    final projection = LibraryTimelineProjection(
      timeline: timeline,
      useAspectRatioWeight: true,
    );

    final lastNewerItem = projection.targetForGlobalItemOffset(899);
    final firstOlderItem = projection.targetForGlobalItemOffset(900);

    expect(lastNewerItem.bucket.monthKey, "2026-08");
    expect(lastNewerItem.itemOffset, 899);
    expect(lastNewerItem.globalItemOffset, 899);
    expect(firstOlderItem.bucket.monthKey, "2022-01");
    expect(firstOlderItem.itemOffset, 0);
    expect(firstOlderItem.globalItemOffset, 900);
  });

  test("retained date positions follow changed bucket membership", () {
    final previous = LibraryTimelineProjection(
      timeline: timeline,
      useAspectRatioWeight: true,
    );
    final current = LibraryTimelineProjection(
      timeline: LibraryTimeline(
        revision: BigInt.one,
        queryId: "query-1",
        totalItems: 300,
        buckets: const [
          LibraryTimeBucket(
            monthKey: "2026-08",
            itemCount: 100,
            aspectRatioSum: 100,
          ),
          LibraryTimeBucket(
            monthKey: "2022-01",
            itemCount: 200,
            aspectRatioSum: 200,
          ),
        ],
      ),
      useAspectRatioWeight: false,
    );
    final oldValue = previous.valueForGlobalItemOffset(925);
    final newValue = current.valueFromProjection(previous, oldValue);
    final target = current.targetForValue(newValue);
    expect(target.bucket.monthKey, "2022-01");
    expect(target.itemOffset, 50);
    expect(
      previous.valueFromProjection(current, newValue),
      closeTo(oldValue, 0.000001),
    );
    expect(current.valueFromProjection(previous, 0), 0);
    expect(current.valueFromProjection(previous, 1), 1);
  });

  test("a removed date resolves to the closest remaining month", () {
    final previous = LibraryTimelineProjection(
      timeline: timeline,
      useAspectRatioWeight: false,
    );
    final current = LibraryTimelineProjection(
      timeline: LibraryTimeline(
        revision: BigInt.one,
        queryId: "query-1",
        totalItems: 20,
        buckets: const [
          LibraryTimeBucket(
            monthKey: "2026-08",
            itemCount: 10,
            aspectRatioSum: 10,
          ),
          LibraryTimeBucket(
            monthKey: "2021-12",
            itemCount: 10,
            aspectRatioSum: 10,
          ),
        ],
      ),
      useAspectRatioWeight: false,
    );
    final target = current.targetForValue(
      current.valueFromProjection(previous, 0.95),
    );
    expect(target.bucket.monthKey, "2021-12");
    expect(target.itemOffset, 0);
  });
}
