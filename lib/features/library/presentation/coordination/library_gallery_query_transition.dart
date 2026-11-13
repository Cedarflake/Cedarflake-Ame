import "dart:async";

import "../../application/library_query_projection.dart";
import "../../application/library_query_refresh.dart";
import "../../domain/library_gallery_group.dart";
import "../../domain/library_models.dart";
import "../../domain/library_query_snapshot.dart";
import "../../domain/library_state.dart";
import "../widgets/library_gallery_wall.dart";
import "library_duplicate_browse_position.dart";
import "library_time_navigation_position.dart";

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
    this.readViewerBrowseAnchor,
  });

  final LibraryState Function() readState;
  final LibraryGalleryVisiblePosition? Function(bool allowPreviousQueryIdentity)
  capturePosition;
  final LibraryAsset? Function() readViewerAnchor;
  final LibraryQueryAnchor? Function(LibraryState, LibraryAsset)?
  readViewerBrowseAnchor;
  final Future<void> Function() reconcileViewer;
  final void Function(LibraryGalleryQueryPublication) onPublished;
  final void Function() onFailed;
  LibraryGalleryVisiblePosition? _pendingPosition;
  LibraryGalleryVisiblePosition? _filterPosition;
  int _generation = 0;
  bool _isDisposed = false;
  Completer<void>? _scrollEnd;

  LibraryGalleryVisiblePosition? get pendingPosition => _pendingPosition;
  bool isCurrent(int generation) => !_isDisposed && generation == _generation;

  @override
  Future<LibraryQueryUpdateOutcome> publish(
    LibraryQueryPublication publication, {
    bool preservePosition = true,
  }) {
    final scrollEnd = _scrollEnd;
    if (preservePosition && scrollEnd != null) {
      return _publishAfterScroll(publication, scrollEnd.future, _generation);
    }
    return _run(
      publication,
      preservePosition: preservePosition,
      reconcileSelection: true,
    );
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
    return _run(publication, reconcileSelection: true);
  }

  void setUserScrolling(bool isScrolling) {
    if (_isDisposed) {
      return;
    }
    if (isScrolling) {
      beginNavigation();
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
    LibraryGalleryVisiblePosition? retainedPosition,
  }) {
    clearFilterPosition();
    return _run(
      publication,
      preservePosition: preservePosition,
      reconcileSelection: reconcileSelection,
      retainedPosition: retainedPosition,
    );
  }

  Future<LibraryQueryUpdateOutcome> runFilter(
    LibraryQueryPublication publication,
  ) {
    if (!_isDisposed) {
      // A fallback or clamped viewport is temporary, not new browsing intent.
      _filterPosition ??= capturePosition(_pendingPosition != null);
    }
    return _run(publication, retainedPosition: _filterPosition);
  }

  void clearFilterPosition() {
    _filterPosition = null;
  }

  void beginNavigation() {
    if (_isDisposed) return;
    clearFilterPosition();
    _generation += 1;
    _pendingPosition = null;
  }

  Future<LibraryQueryUpdateOutcome> _run(
    LibraryQueryPublication publication, {
    bool preservePosition = true,
    bool reconcileSelection = false,
    LibraryGalleryVisiblePosition? retainedPosition,
  }) async {
    if (_isDisposed) {
      return publication(null);
    }
    final generation = ++_generation;
    final before = readState();
    if (!preservePosition) {
      clearFilterPosition();
    }
    final frozen = preservePosition
        ? retainedPosition ??
              capturePosition(_pendingPosition != null) ??
              resolveLibraryTimeNavigationPosition(before)
        : null;
    _pendingPosition = frozen;
    final viewer = reconcileSelection ? readViewerAnchor() : null;
    final locationId = viewer?.locationId ?? frozen?.locationId;
    final viewerIndex = viewer == null
        ? -1
        : before.assets.indexWhere(
            (asset) =>
                asset.assetId == viewer.assetId &&
                asset.galleryEntryId == viewer.galleryEntryId,
          );
    final viewerBrowseAnchor = viewer == null
        ? null
        : readViewerBrowseAnchor?.call(before, viewer);
    final anchor =
        viewerBrowseAnchor ??
        (locationId == null
            ? null
            : LibraryQueryAnchor(
                requestedLocationId: locationId,
                groupKey:
                    viewer?.galleryGroup?.key ??
                    (frozen == null
                        ? null
                        : LibraryGalleryGroupKey.fromEntryId(frozen.entryId)),
                assetId:
                    viewer?.assetId ??
                    frozen?.assetId ??
                    before.assets
                        .where((asset) => asset.locationId == locationId)
                        .firstOrNull
                        ?.assetId,
                fallbackGlobalItemIndex: viewerIndex >= 0
                    ? before.windowStartItemOffset + viewerIndex
                    : frozen?.globalItemIndex ?? 0,
              ));
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
    final current = readState();
    final duplicatePosition = viewer == null
        ? resolveDuplicateBrowsePosition(
            before: before,
            current: current,
            frozen: frozen,
          )
        : null;
    final position =
        duplicatePosition ?? _resolvePosition(current, frozen, anchor);
    _pendingPosition = null;
    onPublished((
      generation: generation,
      position: position,
      preservePosition: preservePosition,
    ));
    return outcome;
  }

  void dispose() {
    clearFilterPosition();
    _isDisposed = true;
    _generation += 1;
    _pendingPosition = null;
    _scrollEnd?.complete();
    _scrollEnd = null;
  }

  LibraryGalleryVisiblePosition? _resolvePosition(
    LibraryState state,
    LibraryGalleryVisiblePosition? frozen,
    LibraryQueryAnchor? requestedAnchor,
  ) {
    final revision = state.catalogRevision;
    if (revision == null || state.assets.isEmpty) {
      return null;
    }
    final loadedIndex = frozen == null
        ? -1
        : state.assets.indexWhere(
            (asset) => asset.galleryEntryId == frozen.entryId,
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
        entryId:
            _assetAtOrdinal(state, resolution.ordinal!)?.galleryEntryId ??
            state.layoutManifest?.entryIdAt(resolution.ordinal!) ??
            resolution.locationId!,
        assetId: _assetAtOrdinal(state, resolution.ordinal!)?.assetId,
        globalItemIndex: resolution.ordinal!,
        itemFraction: frozen.itemFraction,
        viewportFraction: frozen.viewportFraction,
        scrollEdge: frozen.scrollEdge,
      );
    }
    // A viewer-centered read may resolve a different anchor while still loading
    // the wall's location. Its new ordinal comes from the accepted window.
    if (frozen != null &&
        requestedAnchor != null &&
        requestedAnchor.requestedLocationId != frozen.locationId &&
        loadedIndex >= 0) {
      final asset = state.assets[loadedIndex];
      return LibraryGalleryVisiblePosition(
        queryId: state.queryId,
        revision: revision,
        monthKey: null,
        locationId: asset.locationId,
        entryId: asset.galleryEntryId,
        assetId: asset.assetId,
        globalItemIndex: state.windowStartItemOffset + loadedIndex,
        itemFraction: frozen.itemFraction,
        viewportFraction: frozen.viewportFraction,
        scrollEdge: frozen.scrollEdge,
      );
    }
    return LibraryGalleryVisiblePosition(
      queryId: state.queryId,
      revision: revision,
      monthKey: state.timeline?.buckets.firstOrNull?.monthKey,
      locationId: state.assets.first.locationId,
      entryId: state.assets.first.galleryEntryId,
      assetId: state.assets.first.assetId,
      globalItemIndex: state.windowStartItemOffset,
      itemFraction: 0,
      viewportFraction: 0,
      scrollEdge: frozen?.scrollEdge,
    );
  }

  LibraryAsset? _assetAtOrdinal(LibraryState state, int ordinal) {
    final index = ordinal - state.windowStartItemOffset;
    return index >= 0 && index < state.assets.length
        ? state.assets[index]
        : null;
  }
}
