import "../../domain/library_models.dart";
import "library_gallery_layout.dart";
import "library_timeline_projection.dart";

class LibraryTimelineProjectionCache {
  LibraryTimelineProjection? _projection;
  LibraryTimeline? _timeline;

  void clear() {
    _projection = null;
    _timeline = null;
  }

  LibraryTimelineProjection? resolve({
    required LibraryTimeline? timeline,
    required LibraryGalleryLayoutMetrics? metrics,
    required double? maximumScrollOffset,
    required double? viewportExtent,
    required bool useAspectRatioWeight,
  }) {
    if (timeline == null || timeline.buckets.isEmpty) {
      clear();
      return null;
    }
    if (!identical(_timeline, timeline)) {
      clear();
      _timeline = timeline;
    }
    final cached = _projection;
    final hasCompleteLayout =
        metrics != null &&
        metrics.isQueryWide &&
        metrics.itemIndexBase == 0 &&
        metrics.itemCount == timeline.totalItems &&
        maximumScrollOffset != null &&
        viewportExtent != null &&
        ((metrics.contentExtent - viewportExtent).clamp(0.0, double.infinity) -
                    maximumScrollOffset)
                .abs() <
            1;
    if (hasCompleteLayout) {
      if (cached != null &&
          identical(cached.layoutMetrics, metrics) &&
          cached.maximumOffset == maximumScrollOffset) {
        return cached;
      }
      return _projection = LibraryTimelineProjection.fromLayout(
        timeline: timeline,
        metrics: metrics,
        maximumScrollOffset: maximumScrollOffset,
      );
    }
    if (cached != null && cached.layoutMetrics == null) {
      return cached;
    }
    return _projection = LibraryTimelineProjection(
      timeline: timeline,
      useAspectRatioWeight: useAspectRatioWeight,
    );
  }
}
