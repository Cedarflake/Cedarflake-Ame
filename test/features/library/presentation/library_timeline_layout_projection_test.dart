import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_timeline_projection.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_timeline_projection_cache.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  final timeline = LibraryTimeline(
    revision: BigInt.one,
    queryId: "query",
    totalItems: 6,
    buckets: const [
      LibraryTimeBucket(monthKey: "2026-10", itemCount: 3, aspectRatioSum: 90),
      LibraryTimeBucket(monthKey: "2025-01", itemCount: 3, aspectRatioSum: 10),
    ],
  );
  final metrics = _metrics();

  test("uses date headers and rows instead of aspect-ratio aggregates", () {
    final projection = LibraryTimelineProjection.fromLayout(
      timeline: timeline,
      metrics: metrics,
      maximumScrollOffset: 900,
    );
    expect(projection.maximumOffset, 900);
    expect(projection.railBuckets.last.scrollOffset, 350);
    expect(projection.railBuckets.first.contentExtent, 350);
    expect(
      projection.valueForGlobalItemOffset(3),
      closeTo(400 / 900, 0.000001),
    );
    expect(projection.targetForValue(0.5).globalItemOffset, 3);
    expect(projection.targetForValue(350 / 900).bucket.monthKey, "2025-01");
    expect(projection.targetForValue(350 / 900).itemOffset, 0);
  });

  test("reads row targets independently of loaded detail windows", () {
    final projection = LibraryTimelineProjection.fromLayout(
      timeline: timeline,
      metrics: metrics,
      maximumScrollOffset: 900,
    );
    expect(projection.targetForValue(800 / 900).globalItemOffset, 4);
    expect(
      projection.valueForGlobalItemOffset(4),
      projection.valueForGlobalItemOffset(5),
    );
    expect(projection.projection.valueToOffset(0.61), closeTo(549, 0.000001));
  });

  test("reuses exact geometry and recomputes the viewport range", () {
    final cache = LibraryTimelineProjectionCache();
    LibraryTimelineProjection? resolve(double maximum, double viewport) =>
        cache.resolve(
          timeline: timeline,
          metrics: metrics,
          maximumScrollOffset: maximum,
          viewportExtent: viewport,
          useAspectRatioWeight: true,
        );
    final first = resolve(900, 100);
    expect(identical(first, resolve(900, 100)), isTrue);
    final resized = resolve(800, 200);
    expect(identical(first, resized), isFalse);
    expect(resized!.railBuckets.last.scrollOffset, 350);
    expect(resized.projection.offsetToValue(350), closeTo(350 / 800, 0.000001));
  });

  test("does not admit a partial or not-yet-attached layout", () {
    final cache = LibraryTimelineProjectionCache();
    for (final layout in [metrics, _metrics(isQueryWide: false)]) {
      final projection = cache.resolve(
        timeline: timeline,
        metrics: layout,
        maximumScrollOffset: null,
        viewportExtent: null,
        useAspectRatioWeight: true,
      );
      expect(projection!.layoutMetrics, isNull);
      expect(projection.maximumOffset, 100);
    }
    final mismatched = cache.resolve(
      timeline: timeline,
      metrics: metrics,
      maximumScrollOffset: 1200,
      viewportExtent: 100,
      useAspectRatioWeight: true,
    );
    expect(mismatched!.layoutMetrics, isNull);
  });
}

LibraryGalleryLayoutMetrics _metrics({bool isQueryWide = true}) =>
    LibraryGalleryLayoutMetrics(
      contentExtent: 1000,
      photoRowHeight: 200,
      dateAnchors: const [
        LibraryGalleryDateAnchor(
          id: "2026-10-01",
          label: "October",
          scrollOffset: 0,
          year: 2026,
          isUnknown: false,
        ),
        LibraryGalleryDateAnchor(
          id: "2025-01-01",
          label: "January",
          scrollOffset: 350,
          year: 2025,
          isUnknown: false,
        ),
      ],
      locationOffsets: const {},
      itemOffsets: const [50, 50, 50, 400, 800, 800],
      isQueryWide: isQueryWide,
    );
