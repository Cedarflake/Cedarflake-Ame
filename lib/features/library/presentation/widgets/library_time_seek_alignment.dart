import "../../domain/library_models.dart";
import "library_timeline_projection.dart";

/// A semantic target survives its admitted date publication, never old pixels.
class LibraryTimeSeekAlignment {
  const LibraryTimeSeekAlignment({
    required this.generation,
    required this.timeline,
    required this.target,
  });

  final int generation;
  final LibraryTimeline timeline;
  final LibraryTimelineTarget target;

  int? currentItemOffset(
    LibraryTimeline current,
    LibraryTimeAnchor? resolution,
  ) {
    if (current.queryId != timeline.queryId ||
        current.revision < timeline.revision ||
        current.totalItems == 0) {
      return null;
    }
    if (current.revision != timeline.revision &&
        (resolution == null ||
            resolution.revision != current.revision ||
            resolution.queryId != current.queryId)) {
      return null;
    }
    var preceding = 0;
    for (final bucket in current.buckets) {
      if (bucket.monthKey == target.bucket.monthKey && bucket.itemCount > 0) {
        return preceding + target.itemOffset.clamp(0, bucket.itemCount - 1);
      }
      preceding += bucket.itemCount;
    }
    if (resolution == null) {
      return null;
    }
    preceding = 0;
    for (final bucket in current.buckets) {
      if (bucket.monthKey == resolution.monthKey &&
          resolution.itemOffset >= 0 &&
          resolution.itemOffset < bucket.itemCount) {
        return preceding + resolution.itemOffset;
      }
      preceding += bucket.itemCount;
    }
    return null;
  }
}
