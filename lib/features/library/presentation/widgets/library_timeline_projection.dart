import "../../domain/library_models.dart";
import "../library_strings.dart";
import "library_gallery_layout.dart";
import "library_timeline_layout_projection.dart";
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
       _layout = null,
       railBuckets = _buildRailBuckets(timeline, useAspectRatioWeight) {
    projection = TimelineLinearProjection(
      maximumOffset: inferredTimelineMaximumOffset(railBuckets),
    );
  }

  factory LibraryTimelineProjection.fromLayout({
    required LibraryTimeline timeline,
    required LibraryGalleryLayoutMetrics metrics,
    required double maximumScrollOffset,
  }) {
    final layout = LibraryTimelineLayoutProjection(
      timeline: timeline,
      metrics: metrics,
      maximumScrollOffset: maximumScrollOffset,
      buckets: _buildRailBuckets(timeline, false),
    );
    return LibraryTimelineProjection._fromLayout(timeline, layout);
  }

  LibraryTimelineProjection._fromLayout(
    LibraryTimeline timeline,
    LibraryTimelineLayoutProjection layout,
  ) : _timeline = timeline,
      _layout = layout,
      railBuckets = layout.railBuckets {
    projection = layout.projection;
  }

  final LibraryTimeline _timeline;
  final LibraryTimelineLayoutProjection? _layout;
  final List<TimelineRailBucket> railBuckets;
  late final TimelineLinearProjection projection;

  String get queryId => _timeline.queryId;

  double get maximumOffset => projection.maximumOffset;

  LibraryGalleryLayoutMetrics? get layoutMetrics => _layout?.metrics;

  /// Retained ticks identify a date; only this projection owns current offsets.
  double valueFromProjection(LibraryTimelineProjection source, double value) {
    assert(_timeline.queryId == source._timeline.queryId);
    if (identical(this, source) || value <= 0 || value >= 1) {
      return value.clamp(0.0, 1.0).toDouble();
    }
    final target = source.targetForValue(value);
    final sourceOffset = source.globalItemOffsetForValue(value);
    final fraction =
        (target.itemOffset + sourceOffset - sourceOffset.floor()) /
        target.bucket.itemCount;
    var preceding = 0;
    for (final bucket in _timeline.buckets) {
      if (bucket.monthKey == target.bucket.monthKey) {
        return valueForGlobalItemOffset(
          preceding + fraction * bucket.itemCount,
        );
      }
      preceding += bucket.itemCount;
    }
    return valueForGlobalItemOffset(
      _nearestMonthOffset(target.bucket.monthKey).toDouble(),
    );
  }

  int _nearestMonthOffset(String? monthKey) {
    final date = DateTime.tryParse("$monthKey-01");
    final month = date == null ? null : date.year * 12 + date.month;
    int? nearestDistance;
    var nearestOffset = _timeline.totalItems - 1;
    var preceding = 0;
    for (final bucket in _timeline.buckets) {
      final candidate = DateTime.tryParse("${bucket.monthKey}-01");
      if (month != null && candidate != null) {
        final distance = (candidate.year * 12 + candidate.month - month).abs();
        if (nearestDistance == null || distance < nearestDistance) {
          nearestDistance = distance;
          nearestOffset = preceding;
        }
      }
      preceding += bucket.itemCount;
    }
    return nearestOffset;
  }

  double valueForGlobalItemOffset(double globalItemOffset) {
    if (_layout case final layout?) {
      return layout.valueForGlobalItemOffset(globalItemOffset);
    }
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
    return targetForGlobalItemOffset(globalItemOffset);
  }

  LibraryTimelineTarget targetForGlobalItemOffset(int globalItemOffset) {
    assert(_timeline.buckets.isNotEmpty);
    assert(_timeline.totalItems > 0);
    final boundedGlobalItemOffset = globalItemOffset
        .clamp(0, _timeline.totalItems - 1)
        .toInt();
    var precedingItems = 0;
    for (var index = 0; index < _timeline.buckets.length; index++) {
      final source = _timeline.buckets[index];
      final bucketEnd = precedingItems + source.itemCount;
      final isLast = index == _timeline.buckets.length - 1;
      if (boundedGlobalItemOffset < bucketEnd || isLast) {
        final itemOffset = (boundedGlobalItemOffset - precedingItems)
            .clamp(0, source.itemCount - 1)
            .toInt();
        return LibraryTimelineTarget(
          bucket: source,
          itemOffset: itemOffset,
          globalItemOffset: boundedGlobalItemOffset,
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
    if (_layout case final layout?) {
      return layout.globalItemOffsetForValue(value);
    }
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
