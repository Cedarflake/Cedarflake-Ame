import "../../domain/library_models.dart";
import "library_gallery_layout.dart";
import "timeline_linear_projection.dart";
import "timeline_rail_model.dart";

class LibraryTimelineLayoutProjection {
  LibraryTimelineLayoutProjection({
    required this.metrics,
    required double maximumScrollOffset,
    required LibraryTimeline timeline,
    required List<TimelineRailBucket> buckets,
  }) : projection = TimelineLinearProjection(
         maximumOffset: maximumScrollOffset,
       ) {
    final monthOffsets = <String, double>{};
    for (final anchor in metrics.dateAnchors) {
      if (!anchor.isUnknown && anchor.id.length < 7) {
        continue;
      }
      final month = anchor.isUnknown ? "unknown" : anchor.id.substring(0, 7);
      monthOffsets.putIfAbsent(month, () => anchor.scrollOffset);
    }
    var itemOffset = 0;
    final starts = <({TimelineRailBucket bucket, double offset})>[];
    for (var index = 0; index < buckets.length; index++) {
      final bucket = buckets[index];
      final offset = index == 0
          ? 0.0
          : monthOffsets[bucket.id] ??
                metrics.offsetForGlobalItemIndex(itemOffset) ??
                metrics.contentExtent;
      starts.add((bucket: bucket, offset: offset));
      itemOffset += timeline.buckets[index].itemCount;
    }
    final dates = <({double offset, int itemOffset})>[];
    var nextItem = 0;
    for (final anchor in metrics.dateAnchors) {
      while (nextItem < metrics.itemCount &&
          metrics.itemOffsets[nextItem] < anchor.scrollOffset) {
        nextItem++;
      }
      if (nextItem < metrics.itemCount) {
        dates.add((
          offset: anchor.scrollOffset,
          itemOffset: metrics.itemIndexBase + nextItem,
        ));
      }
    }
    _dateStarts = List.unmodifiable(dates);
    railBuckets = List.unmodifiable([
      for (var index = 0; index < starts.length; index++)
        TimelineRailBucket(
          id: starts[index].bucket.id,
          label: starts[index].bucket.label,
          contentExtent:
              (index + 1 < starts.length
                      ? starts[index + 1].offset - starts[index].offset
                      : metrics.contentExtent - starts[index].offset)
                  .clamp(0.0, double.infinity)
                  .toDouble(),
          scrollOffset: starts[index].offset,
          year: starts[index].bucket.year,
          isUnknown: starts[index].bucket.isUnknown,
        ),
    ]);
  }

  final LibraryGalleryLayoutMetrics metrics;
  final TimelineLinearProjection projection;
  late final List<TimelineRailBucket> railBuckets;
  late final List<({double offset, int itemOffset})> _dateStarts;

  double valueForGlobalItemOffset(double itemOffset) {
    final offset = metrics.navigationOffsetForGlobalItemIndex(
      itemOffset.floor(),
    );
    return projection.offsetToValue(offset ?? metrics.contentExtent);
  }

  double globalItemOffsetForValue(double value) {
    final offset = projection.valueToOffset(value);
    var itemOffset =
        metrics.itemIndexBase + metrics.itemIndexForScrollOffset(offset);
    var lower = 0;
    var upper = _dateStarts.length;
    while (lower < upper) {
      final middle = lower + ((upper - lower) >> 1);
      if (_dateStarts[middle].offset <= offset + 0.000001) {
        lower = middle + 1;
      } else {
        upper = middle;
      }
    }
    if (lower > 0 && _dateStarts[lower - 1].itemOffset > itemOffset) {
      itemOffset = _dateStarts[lower - 1].itemOffset;
    }
    return itemOffset.toDouble();
  }
}
