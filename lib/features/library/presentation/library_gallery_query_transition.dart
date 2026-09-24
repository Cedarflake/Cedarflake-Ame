import "dart:async";

import "../application/library_query_projection.dart";
import "../application/library_query_refresh.dart";
import "../domain/library_models.dart";
import "../domain/library_query_snapshot.dart";
import "../domain/library_state.dart";
import "widgets/library_gallery_wall.dart";

typedef LibraryGalleryQueryPublication = ({
  int generation,
  LibraryGalleryVisiblePosition? position,
  bool preservePosition,
});

/// Owns capture, stable-identity resolution and restoration of a query position.
/// The application separately decides whether the catalog read may publish.
class LibraryGalleryQueryTransition implements LibraryQueryProjection {
  LibraryGalleryQueryTransition({
    required this.readState,
    required this.capturePosition,
    required this.readViewerAnchor,
    required this.reconcileViewer,
    required this.onPublished,
    required this.onFailed,
  });

  final LibraryState Function() readState;
  final LibraryGalleryVisiblePosition? Function(bool allowPreviousQueryIdentity)
  capturePosition;
  final LibraryAsset? Function() readViewerAnchor;
  final Future<void> Function() reconcileViewer;
  final void Function(LibraryGalleryQueryPublication) onPublished;
  final void Function() onFailed;
  LibraryGalleryVisiblePosition? _pendingPosition;
  int _generation = 0;
  bool _isDisposed = false;
  Completer<void>? _scrollEnd;

  LibraryGalleryVisiblePosition? get pendingPosition => _pendingPosition;
  bool isCurrent(int generation) => !_isDisposed && generation == _generation;

  @override
  Future<LibraryQueryUpdateOutcome> publish(
    LibraryQueryPublication publication,
  ) {
    final scrollEnd = _scrollEnd;
    if (scrollEnd != null) {
      return _publishAfterScroll(publication, scrollEnd.future, _generation);
    }
    return run(publication, reconcileSelection: true);
  }

  Future<LibraryQueryUpdateOutcome> _publishAfterScroll(
    LibraryQueryPublication publication,
    Future<void> scrollEnd,
    int generation,
  ) async {
    await scrollEnd;
    if (!isCurrent(generation)) {
      return LibraryQueryUpdateOutcome.superseded;
    }
    return run(publication, reconcileSelection: true);
  }

  void setUserScrolling(bool isScrolling) {
    if (_isDisposed) {
      return;
    }
    if (isScrolling) {
      _generation += 1;
      _pendingPosition = null;
      _scrollEnd ??= Completer<void>();
    } else {
      _scrollEnd?.complete();
      _scrollEnd = null;
    }
  }

  Future<LibraryQueryUpdateOutcome> run(
    LibraryQueryPublication publication, {
    bool preservePosition = true,
    bool reconcileSelection = false,
  }) async {
    if (_isDisposed) {
      return publication(null);
    }
    final generation = ++_generation;
    final before = readState();
    final frozen = preservePosition
        ? capturePosition(_pendingPosition != null)
        : null;
    _pendingPosition = frozen;
    final viewer = reconcileSelection ? readViewerAnchor() : null;
    final locationId = viewer?.locationId ?? frozen?.locationId;
    final viewerIndex = viewer == null
        ? -1
        : before.assets.indexWhere(
            (asset) =>
                asset.assetId == viewer.assetId &&
                asset.locationId == viewer.locationId,
          );
    final anchor = locationId == null
        ? null
        : LibraryQueryAnchor(
            requestedLocationId: locationId,
            assetId:
                viewer?.assetId ??
                frozen?.assetId ??
                _assetAt(before, locationId)?.assetId,
            fallbackGlobalItemIndex: viewerIndex >= 0
                ? before.windowStartItemOffset + viewerIndex
                : frozen?.globalItemIndex ?? 0,
          );
    final LibraryQueryUpdateOutcome outcome;
    try {
      outcome = await publication(anchor);
    } on Object {
      if (isCurrent(generation)) {
        _pendingPosition = null;
        onFailed();
      }
      rethrow;
    }
    if (!isCurrent(generation)) {
      return outcome;
    }
    if (outcome != LibraryQueryUpdateOutcome.applied) {
      _pendingPosition = null;
      onFailed();
      return outcome;
    }
    if (reconcileSelection) {
      await reconcileViewer();
      if (!isCurrent(generation)) {
        return outcome;
      }
    }
    final position = _resolvePosition(readState(), frozen);
    _pendingPosition = null;
    onPublished((
      generation: generation,
      position: position,
      preservePosition: preservePosition,
    ));
    return outcome;
  }

  void dispose() {
    _isDisposed = true;
    _generation += 1;
    _pendingPosition = null;
    _scrollEnd?.complete();
    _scrollEnd = null;
  }

  LibraryGalleryVisiblePosition? _resolvePosition(
    LibraryState state,
    LibraryGalleryVisiblePosition? frozen,
  ) {
    final revision = state.catalogRevision;
    if (revision == null || state.assets.isEmpty) {
      return null;
    }
    final loadedIndex = frozen == null
        ? -1
        : state.assets.indexWhere(
            (asset) => asset.locationId == frozen.locationId,
          );
    if (frozen != null &&
        frozen.queryId == state.queryId &&
        frozen.revision == revision &&
        loadedIndex >= 0 &&
        state.windowStartItemOffset + loadedIndex == frozen.globalItemIndex) {
      return frozen;
    }
    final resolution = state.queryAnchorResolution;
    if (frozen != null &&
        resolution != null &&
        resolution.requestedLocationId == frozen.locationId &&
        resolution.didResolve) {
      return LibraryGalleryVisiblePosition(
        queryId: state.queryId,
        revision: revision,
        monthKey: null,
        locationId: resolution.locationId!,
        assetId: _assetAt(state, resolution.locationId!)?.assetId,
        globalItemIndex: resolution.ordinal!,
        itemFraction: frozen.itemFraction,
        viewportFraction: frozen.viewportFraction,
      );
    }
    return LibraryGalleryVisiblePosition(
      queryId: state.queryId,
      revision: revision,
      monthKey: state.timeline?.buckets.firstOrNull?.monthKey,
      locationId: state.assets.first.locationId,
      assetId: state.assets.first.assetId,
      globalItemIndex: state.windowStartItemOffset,
      itemFraction: 0,
      viewportFraction: 0,
    );
  }

  LibraryAsset? _assetAt(LibraryState state, String locationId) {
    for (final asset in state.assets) {
      if (asset.locationId == locationId) {
        return asset;
      }
    }
    return null;
  }
}
