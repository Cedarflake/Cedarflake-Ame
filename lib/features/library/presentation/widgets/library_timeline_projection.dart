import "../../domain/library_models.dart";
import "../library_strings.dart";
import "timeline_linear_projection.dart";
import "timeline_rail_model.dart";

class LibraryTimelineTarget {
  const LibraryTimelineTarget({
    required this.bucket,
    required this.itemOffset,
    required this.globalItemOffset,
  });

  final LibraryTimeBucket bucket;
  final int itemOffset;
  final int globalItemOffset;
}

class LibraryTimelineProjection {
  LibraryTimelineProjection({
    required LibraryTimeline timeline,
    required bool useAspectRatioWeight,
  }) : _timeline = timeline,
       railBuckets = _buildRailBuckets(timeline, useAspectRatioWeight) {
    projection = TimelineLinearProjection(
      maximumOffset: inferredTimelineMaximumOffset(railBuckets),
    );
  }

  final LibraryTimeline _timeline;
  final List<TimelineRailBucket> railBuckets;
  late final TimelineLinearProjection projection;

  double get maximumOffset => projection.maximumOffset;

  double valueForGlobalItemOffset(double globalItemOffset) {
    if (_timeline.buckets.isEmpty || _timeline.totalItems <= 0) {
      return 0;
    }
    var remainingItems = globalItemOffset
        .clamp(0.0, _timeline.totalItems.toDouble())
        .toDouble();
    var precedingWeight = 0.0;
    for (var index = 0; index < _timeline.buckets.length; index++) {
      final source = _timeline.buckets[index];
      final rail = railBuckets[index];
      final isLast = index == _timeline.buckets.length - 1;
      if (remainingItems < source.itemCount || isLast) {
        final fraction = source.itemCount <= 0
            ? 0.0
            : (remainingItems / source.itemCount).clamp(0.0, 1.0).toDouble();
        return projection.offsetToValue(
          precedingWeight + (rail.contentExtent * fraction),
        );
      }
      remainingItems -= source.itemCount;
      precedingWeight += rail.contentExtent;
    }
    return 1;
  }

  LibraryTimelineTarget targetForValue(double value) {
    assert(_timeline.buckets.isNotEmpty);
    assert(_timeline.totalItems > 0);
    final globalItemOffset = globalItemOffsetForValue(
      value,
    ).floor().clamp(0, _timeline.totalItems - 1).toInt();
    var precedingItems = 0;
    for (var index = 0; index < _timeline.buckets.length; index++) {
      final source = _timeline.buckets[index];
      final bucketEnd = precedingItems + source.itemCount;
      final isLast = index == _timeline.buckets.length - 1;
      if (globalItemOffset < bucketEnd || isLast) {
        final itemOffset = (globalItemOffset - precedingItems)
            .clamp(0, source.itemCount - 1)
            .toInt();
        return LibraryTimelineTarget(
          bucket: source,
          itemOffset: itemOffset,
          globalItemOffset: globalItemOffset,
        );
      }
      precedingItems = bucketEnd;
    }
    final last = _timeline.buckets.last;
    return LibraryTimelineTarget(
      bucket: last,
      itemOffset: (last.itemCount - 1).clamp(0, last.itemCount).toInt(),
      globalItemOffset: (_timeline.totalItems - 1)
          .clamp(0, _timeline.totalItems)
          .toInt(),
    );
  }

  double globalItemOffsetForValue(double value) {
    if (_timeline.buckets.isEmpty || _timeline.totalItems <= 0) {
      return 0;
    }
    final targetWeight = projection.valueToOffset(value);
    var precedingWeight = 0.0;
    var precedingItems = 0;
    for (var index = 0; index < _timeline.buckets.length; index++) {
      final source = _timeline.buckets[index];
      final rail = railBuckets[index];
      final bucketEnd = precedingWeight + rail.contentExtent;
      final isLast = index == _timeline.buckets.length - 1;
      if (targetWeight < bucketEnd || isLast) {
        final fraction = rail.contentExtent <= 0
            ? 0.0
            : ((targetWeight - precedingWeight) / rail.contentExtent)
                  .clamp(0.0, 1.0)
                  .toDouble();
        return _stabilizeItemOffset(
          precedingItems + (fraction * source.itemCount),
        );
      }
      precedingWeight = bucketEnd;
      precedingItems += source.itemCount;
    }
    return _timeline.totalItems.toDouble();
  }

  double _stabilizeItemOffset(double value) {
    final nearestItem = value.roundToDouble();
    return (value - nearestItem).abs() <= 0.000001 ? nearestItem : value;
  }

  static List<TimelineRailBucket> _buildRailBuckets(
    LibraryTimeline timeline,
    bool useAspectRatioWeight,
  ) {
    return List.unmodifiable([
      for (final bucket in timeline.buckets)
        TimelineRailBucket(
          id: bucket.monthKey ?? "unknown",
          label: _monthLabel(bucket.monthKey),
          contentExtent: useAspectRatioWeight && bucket.aspectRatioSum > 0
              ? bucket.aspectRatioSum
              : bucket.itemCount.toDouble(),
          year: bucket.monthKey == null || bucket.monthKey!.length < 4
              ? null
              : int.tryParse(bucket.monthKey!.substring(0, 4)),
          isUnknown: bucket.isUnknown,
        ),
    ]);
  }

  static String _monthLabel(String? monthKey) {
    if (monthKey == null) {
      return LibraryStrings.unknownCaptureDate;
    }
    final parts = monthKey.split("-");
    if (parts.length != 2) {
      return monthKey;
    }
    final month = int.tryParse(parts[1]);
    return month == null ? monthKey : "${parts[0]}年$month月";
  }
}
