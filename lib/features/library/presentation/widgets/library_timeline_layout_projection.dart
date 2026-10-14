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
    final starts =
        <({TimelineRailBucket bucket, double offset, int itemOffset})>[];
    for (var index = 0; index < buckets.length; index++) {
      final bucket = buckets[index];
      final offset =
          monthOffsets[bucket.id] ??
          metrics.offsetForGlobalItemIndex(itemOffset) ??
          metrics.contentExtent;
      starts.add((bucket: bucket, offset: offset, itemOffset: itemOffset));
      itemOffset += timeline.buckets[index].itemCount;
    }
    _monthStarts = List.unmodifiable([
      for (final start in starts)
        (offset: start.offset, itemOffset: start.itemOffset),
    ]);
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
  late final List<({double offset, int itemOffset})> _monthStarts;

  double valueForGlobalItemOffset(double itemOffset) {
    final offset = metrics.offsetForGlobalItemIndex(itemOffset.floor());
    return projection.offsetToValue(offset ?? metrics.contentExtent);
  }

  double globalItemOffsetForValue(double value) {
    final offset = projection.valueToOffset(value);
    var itemOffset =
        metrics.itemIndexBase + metrics.itemIndexForScrollOffset(offset);
    for (final month in _monthStarts) {
      if (month.offset > offset + 0.000001) {
        break;
      }
      if (month.itemOffset > itemOffset) {
        itemOffset = month.itemOffset;
      }
    }
    return itemOffset.toDouble();
  }
}
