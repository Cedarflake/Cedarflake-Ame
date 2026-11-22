import "../../../../app/presentation/ame_localizations.dart";
import "../../domain/library_models.dart";
import "../library_gallery_date.dart";
import "library_gallery_layout.dart";
import "library_timeline_projection.dart";

class LibraryTimelineProjectionCache {
  LibraryTimelineProjection? _projection;
  LibraryTimeline? _timeline;
  String? _locale;

  LibraryTimelineProjection? get current => _projection;

  void clear() {
    _projection = null;
    _timeline = null;
    _locale = null;
  }

  LibraryTimelineProjection? resolve(
    AppLocalizations strings, {
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
    if (!identical(_timeline, timeline) || _locale != strings.localeName) {
      clear();
      _timeline = timeline;
      _locale = strings.localeName;
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
        strings,
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
      monthLabel: (key) => galleryDateLabel(strings, key, monthOnly: true),
    );
  }
}
