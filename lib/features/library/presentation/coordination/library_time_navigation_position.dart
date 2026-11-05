import "../../domain/library_state.dart";
import "../widgets/library_gallery_position.dart";

/// Bridges an admitted date result to position capture before its first layout.
LibraryGalleryVisiblePosition? resolveLibraryTimeNavigationPosition(
  LibraryState state,
) {
  final anchor = state.activeTimeAnchor;
  final timeline = state.timeline;
  if (anchor == null ||
      timeline == null ||
      anchor.queryId != state.queryId ||
      anchor.revision != state.catalogRevision ||
      timeline.queryId != anchor.queryId ||
      timeline.revision != anchor.revision) {
    return null;
  }
  var preceding = 0;
  for (final bucket in timeline.buckets) {
    if (bucket.monthKey == anchor.monthKey) {
      if (anchor.itemOffset < 0 || anchor.itemOffset >= bucket.itemCount) {
        return null;
      }
      final globalIndex = preceding + anchor.itemOffset;
      final localIndex = globalIndex - state.windowStartItemOffset;
      if (localIndex < 0 || localIndex >= state.assets.length) return null;
      final asset = state.assets[localIndex];
      return LibraryGalleryVisiblePosition(
        queryId: anchor.queryId,
        revision: anchor.revision,
        monthKey: anchor.monthKey,
        locationId: asset.locationId,
        assetId: asset.assetId,
        globalItemIndex: globalIndex,
        itemFraction: 0,
        viewportFraction: 0,
      );
    }
    preceding += bucket.itemCount;
  }
  return null;
}
