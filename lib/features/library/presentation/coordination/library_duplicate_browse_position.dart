import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "../widgets/library_gallery_position.dart";

/// A merged representative can have a different date from the browsed copy.
/// Keep chronological browsing near that copy instead of following its group.
LibraryGalleryVisiblePosition? resolveDuplicateBrowsePosition({
  required LibraryState before,
  required LibraryState current,
  required LibraryGalleryVisiblePosition? frozen,
}) {
  final previous = before.layoutManifest;
  final next = current.layoutManifest;
  final resolution = current.queryAnchorResolution;
  if (frozen == null ||
      previous == null ||
      next == null ||
      next.itemCount == 0 ||
      !current.query.isChronological ||
      !current.query.foldsDuplicates ||
      before.query.copyWith(duplicateMode: LibraryGalleryDuplicateMode.all) !=
          current.query.copyWith(
            duplicateMode: LibraryGalleryDuplicateMode.all,
          ) ||
      previous.queryId != frozen.queryId ||
      previous.revision != frozen.revision ||
      next.queryId != current.queryId ||
      next.revision != current.catalogRevision ||
      frozen.globalItemIndex < 0 ||
      frozen.globalItemIndex >= previous.itemCount ||
      previous.locationIdAt(frozen.globalItemIndex) != frozen.locationId ||
      resolution == null ||
      resolution.requestedLocationId != frozen.locationId ||
      !resolution.didResolve ||
      resolution.locationId == frozen.locationId) {
    return null;
  }
  final oldTimeline = before.timeline;
  final newTimeline = current.timeline;
  final hasNewMerges =
      oldTimeline != null &&
      newTimeline != null &&
      newTimeline.totalSourceItems >= oldTimeline.totalSourceItems &&
      newTimeline.totalSourceItems - newTimeline.totalItems >
          oldTimeline.totalSourceItems - oldTimeline.totalItems;
  if (before.query.foldsDuplicates && !hasNewMerges) return null;
  final originalDate = previous.dateKeyAt(frozen.globalItemIndex);
  final originalDay = originalDate == null
      ? null
      : DateTime.tryParse(originalDate);
  int? nearest;
  int? distance;
  String? lastDate;
  for (var index = 0; index < next.itemCount; index++) {
    final date = next.dateKeyAt(index);
    if (date == originalDate) {
      nearest = index;
      break;
    }
    if (date == null || date == lastDate || originalDay == null) continue;
    lastDate = date;
    final day = DateTime.tryParse(date);
    if (day == null) continue;
    final candidateDistance = day.difference(originalDay).inDays.abs();
    if (distance == null || candidateDistance < distance) {
      distance = candidateDistance;
      nearest = index;
    }
  }
  if (nearest == null) return null;
  final locationId = next.locationIdAt(nearest);
  final date = next.dateKeyAt(nearest);
  return LibraryGalleryVisiblePosition(
    queryId: next.queryId,
    revision: next.revision,
    monthKey: date != null && date.length >= 7 ? date.substring(0, 7) : null,
    locationId: locationId,
    assetId: null,
    globalItemIndex: nearest,
    itemFraction: frozen.itemFraction,
    viewportFraction: frozen.viewportFraction,
    scrollEdge: frozen.scrollEdge,
  );
}
