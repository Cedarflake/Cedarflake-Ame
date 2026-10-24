import "../application/library_query_refresh.dart";
import "../domain/library_models.dart";
import "../domain/library_organization.dart";
import "../domain/library_state.dart";
import "library_gallery_query_transition.dart";
import "widgets/library_gallery_position.dart";

typedef LibraryReviewQueryUpdate =
    Future<bool> Function(
      LibraryGalleryQuery query, {
      String? anchorLocationId,
      String? anchorAssetId,
      int? fallbackGlobalItemIndex,
      required bool forceRefresh,
    });

/// Restores review targets or resets ordinary browsing through the shared viewport.
class LibraryReviewQueryTransition {
  const LibraryReviewQueryTransition({
    required this.readState,
    required this.updateQuery,
    required this.positionTransition,
  });

  final LibraryState Function() readState;
  final LibraryReviewQueryUpdate updateQuery;
  final LibraryGalleryQueryTransition positionTransition;

  Future<bool> show(
    LibraryGalleryQuery query, {
    LibraryOrganizationTarget? target,
    LibraryGalleryVisiblePosition? position,
  }) async {
    final state = readState();
    final anchorPosition = target == null
        ? position
        : LibraryGalleryVisiblePosition(
            queryId: state.queryId,
            revision: state.catalogRevision ?? BigInt.zero,
            monthKey: null,
            locationId: target.locationId,
            assetId: target.assetId,
            globalItemIndex: 0,
            itemFraction: 0,
            viewportFraction: 0,
          );
    final outcome = await positionTransition.run(
      (anchor) async {
        final applied = await updateQuery(
          query,
          anchorLocationId: anchor?.requestedLocationId,
          anchorAssetId: anchor?.assetId,
          fallbackGlobalItemIndex: anchor?.fallbackGlobalItemIndex,
          forceRefresh: anchorPosition != null || !_canReuseStart(readState()),
        );
        return applied
            ? LibraryQueryUpdateOutcome.applied
            : LibraryQueryUpdateOutcome.failed;
      },
      preservePosition: anchorPosition != null,
      retainedPosition: anchorPosition,
    );
    return outcome == LibraryQueryUpdateOutcome.applied;
  }

  bool _canReuseStart(LibraryState state) {
    if (state.windowStartItemOffset != 0 || state.activeTimeAnchor != null) {
      return false;
    }
    // An in-flight window must retire through the application's refresh path.
    if (state.queryActivity is! LibraryQueryIdle ||
        state.isLoadingPage ||
        state.isLoadingPreviousPage ||
        state.isLoadingTimeAnchor ||
        state.isLoadingVisibleRange) {
      return false;
    }
    return true;
  }
}
