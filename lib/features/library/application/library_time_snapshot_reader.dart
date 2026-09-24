import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_catalog.dart";
import "library_time_navigation_requests.dart";

class LibraryTimeSnapshotReader {
  const LibraryTimeSnapshotReader(this._catalog);

  final LibraryCatalog _catalog;

  Future<LibraryTimeSnapshot?> load(
    LibraryTimeNavigationRequest request, {
    required bool Function() canPublish,
    required bool Function() ownsExplicitIntent,
  }) async {
    try {
      final snapshot = await _catalog.loadAtTime(
        maxItems: libraryTimelineWindow,
        query: request.query,
        anchor: request.anchor,
      );
      if (!canPublish()) {
        return null;
      }
      if (snapshot.revision != request.timeline.revision ||
          snapshot.queryId != request.timeline.queryId) {
        throw const LibraryCatalogFailure(
          code: "catalog_cursor_stale",
          message: "The catalog changed while navigating the timeline",
        );
      }
      return LibraryTimeSnapshot(
        snapshot: snapshot,
        timeline: request.timeline,
        anchor: request.anchor,
        windowStartItemOffset: request.globalItemOffset,
      );
    } on LibraryCatalogFailure catch (error) {
      if (!canPublish()) {
        return null;
      }
      if (error.code != "catalog_cursor_stale" || !ownsExplicitIntent()) {
        rethrow;
      }
      final result = await _catalog.resolveTimeIntent(
        maxItems: libraryTimelineWindow,
        query: request.query,
        intent: LibraryTimeIntent(
          monthKey: request.anchor.monthKey,
          itemOffset: request.anchor.itemOffset,
        ),
      );
      if (!canPublish() || !ownsExplicitIntent()) {
        return null;
      }
      _validateResolution(result, request);
      return result;
    }
  }

  void _validateResolution(
    LibraryTimeSnapshot result,
    LibraryTimeNavigationRequest request,
  ) {
    final snapshot = result.snapshot;
    final timeline = result.timeline;
    final anchor = result.anchor;
    final rootId = request.query.rootId;
    if (rootId != null && !snapshot.roots.any((root) => root.id == rootId)) {
      throw const LibraryCatalogFailure(
        code: "catalog_time_target_removed",
        message: "The selected library root is no longer available",
      );
    }
    final targetOffset = result.targetItemOffset;
    final windowStart = result.windowStartItemOffset;
    final windowEnd = windowStart + snapshot.assets.length;
    final isEmpty =
        timeline.totalItems == 0 &&
        snapshot.assets.isEmpty &&
        anchor == null &&
        result.windowStartItemOffset == 0;
    final isResolved =
        anchor != null &&
        targetOffset != null &&
        anchor.revision == timeline.revision &&
        anchor.queryId == timeline.queryId &&
        windowStart >= 0 &&
        targetOffset >= windowStart &&
        targetOffset < windowEnd &&
        snapshot.assets.isNotEmpty &&
        windowEnd <= timeline.totalItems;
    final bucketCount = timeline.buckets.fold<int>(
      0,
      (total, bucket) => total + bucket.itemCount,
    );
    bool matchesCursor(LibraryCatalogCursor? cursor) =>
        cursor == null ||
        (cursor.revision == snapshot.revision &&
            cursor.queryId == snapshot.queryId);
    if (snapshot.revision != timeline.revision ||
        snapshot.revision < request.timeline.revision ||
        snapshot.queryId != timeline.queryId ||
        timeline.queryId != request.timeline.queryId ||
        bucketCount != timeline.totalItems ||
        !matchesCursor(snapshot.previousCursor) ||
        !matchesCursor(snapshot.nextCursor) ||
        snapshot.assets.length > libraryTimelineWindow ||
        (!isEmpty && !isResolved)) {
      throw const LibraryCatalogFailure(
        code: "catalog_time_snapshot_invalid",
        message: "The date result has inconsistent page and timeline evidence",
      );
    }
  }
}

LibraryState publishLibraryTimeSnapshot(
  LibraryState current,
  LibraryTimeSnapshot result,
) => current.copyWith(
  roots: result.snapshot.roots,
  assets: result.snapshot.assets,
  catalogPath: result.snapshot.catalogPath,
  catalogRevision: result.snapshot.revision,
  queryId: result.snapshot.queryId,
  timeline: result.timeline,
  windowStartItemOffset: result.windowStartItemOffset,
  previousCursor: result.snapshot.previousCursor,
  nextCursor: result.snapshot.nextCursor,
  activeTimeAnchor: result.anchor,
  isLoadingTimeAnchor: false,
  pageErrorMessage: null,
);
