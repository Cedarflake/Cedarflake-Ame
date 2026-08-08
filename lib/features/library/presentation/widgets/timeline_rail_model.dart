import "timeline_linear_projection.dart";

class TimelineRailBucket {
  const TimelineRailBucket({
    required this.id,
    required this.label,
    required this.contentExtent,
    required this.year,
    this.scrollOffset,
    this.isUnknown = false,
  });

  final String id;
  final String label;
  final double contentExtent;
  final double? scrollOffset;
  final int? year;
  final bool isUnknown;
}

class TimelineRailAnchor {
  const TimelineRailAnchor({
    required this.bucket,
    required this.value,
    required this.isYearStart,
  });

  final TimelineRailBucket bucket;
  final double value;
  final bool isYearStart;
}

List<TimelineRailAnchor> buildTimelineRailAnchors(
  List<TimelineRailBucket> buckets,
  TimelineLinearProjection projection,
) {
  if (buckets.isEmpty) {
    return const [];
  }
  final anchors = <TimelineRailAnchor>[];
  int? previousYear;
  var runningOffset = 0.0;
  for (final bucket in buckets) {
    final offset = bucket.scrollOffset ?? runningOffset;
    anchors.add(
      TimelineRailAnchor(
        bucket: bucket,
        value: projection.offsetToValue(offset),
        isYearStart: bucket.year != previousYear,
      ),
    );
    previousYear = bucket.year;
    runningOffset += bucket.contentExtent;
  }
  return List.unmodifiable(anchors);
}

TimelineRailAnchor timelineRailBucketAtValue(
  List<TimelineRailAnchor> anchors,
  double value,
) {
  var lower = 0;
  var upper = anchors.length - 1;
  while (lower <= upper) {
    final middle = lower + ((upper - lower) >> 1);
    if (anchors[middle].value <= value) {
      lower = middle + 1;
    } else {
      upper = middle - 1;
    }
  }
  return anchors[upper.clamp(0, anchors.length - 1)];
}

double timelineRailValueForBucket(
  List<TimelineRailBucket> buckets,
  String bucketId, {
  double? maximumScrollOffset,
}) {
  if (buckets.isEmpty) {
    return 0;
  }
  final projection = TimelineLinearProjection(
    maximumOffset:
        maximumScrollOffset ?? inferredTimelineMaximumOffset(buckets),
  );
  final anchors = buildTimelineRailAnchors(buckets, projection);
  for (final anchor in anchors) {
    if (anchor.bucket.id == bucketId) {
      return anchor.value;
    }
  }
  return 0;
}

double inferredTimelineMaximumOffset(List<TimelineRailBucket> buckets) {
  if (buckets.isEmpty) {
    return 0;
  }
  if (buckets.every((bucket) => bucket.scrollOffset != null)) {
    return (buckets.last.scrollOffset ?? 0) + buckets.last.contentExtent;
  }
  return buckets.fold<double>(
    0,
    (extent, bucket) => extent + bucket.contentExtent,
  );
}
