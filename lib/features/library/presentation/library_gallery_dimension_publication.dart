import "dart:async";

import "../domain/gallery_layout_manifest.dart";
import "../domain/library_state.dart";
import "widgets/library_gallery_wall.dart";

/// Publishes recovered dimensions within one query, revision and visible-range epoch.
class LibraryGalleryDimensionPublication {
  LibraryGalleryDimensionPublication({
    required LibraryState Function() readState,
    required LibraryGalleryVisiblePosition? Function(LibraryState state)
    capturePosition,
    required bool Function() hasPendingTransition,
    required void Function(bool isScrolling) onUserScrollingChanged,
    required void Function(LibraryGalleryVisiblePosition? anchor) onPublished,
  }) : _readState = readState,
       _capturePosition = capturePosition,
       _hasPendingTransition = hasPendingTransition,
       _onUserScrollingChanged = onUserScrollingChanged,
       _onPublished = onPublished;

  static const _layoutDimensionQuietDelay = Duration(milliseconds: 160);
  static const _layoutDimensionMaximumDelay = Duration(milliseconds: 600);

  final LibraryState Function() _readState;
  final LibraryGalleryVisiblePosition? Function(LibraryState state)
  _capturePosition;
  final bool Function() _hasPendingTransition;
  final void Function(bool isScrolling) _onUserScrollingChanged;
  final void Function(LibraryGalleryVisiblePosition? anchor) _onPublished;
  Timer? _layoutDimensionSettleTimer;
  Timer? _layoutDimensionDeadlineTimer;
  final Map<int, LibraryGalleryLayoutDimensionUpdate>
  _pendingLayoutDimensionUpdates = {};
  final Map<int, LibraryGalleryLayoutDimensionUpdate>
  _deferredLayoutDimensionUpdates = {};
  final Map<int, LibraryGalleryLayoutDimensionUpdate>
  _publishedLayoutDimensionUpdates = {};
  LibraryGalleryLayoutManifest? _dimensionUpdateBaseManifest;
  LibraryGalleryLayoutManifest? _dimensionUpdatedManifest;
  BigInt? _layoutDimensionRevision;
  String _layoutDimensionQueryId = "";
  LibraryGalleryVisibleRange? _visibleGalleryRange;
  LibraryGalleryVisibleRange? _layoutDimensionRecoveryRange;
  LibraryGalleryVisiblePosition? _layoutDimensionRecoveryAnchor;
  bool _isUserScrolling = false;
  bool _isDisposed = false;
  int _dimensionUpdateGeneration = 0;
  int _dimensionUpdatedManifestGeneration = -1;

  void dispose() {
    _isDisposed = true;
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionDeadlineTimer?.cancel();
  }

  void resetVisibleRange() {
    _visibleGalleryRange = null;
    _layoutDimensionRecoveryRange = null;
    _layoutDimensionRecoveryAnchor = null;
  }

  void beginNavigation() {
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionSettleTimer = null;
    _layoutDimensionDeadlineTimer?.cancel();
    _layoutDimensionDeadlineTimer = null;
    resetVisibleRange();
  }

  void accept(LibraryGalleryLayoutDimensionUpdate update) {
    if (_isDisposed) {
      return;
    }
    final current = _readState();
    if (current.catalogRevision != update.revision ||
        current.queryId != update.queryId) {
      return;
    }
    synchronizeContext(update.revision, update.queryId);
    if (!_isUserScrolling) {
      _ensureLayoutDimensionRecoveryEpoch(current);
    }
    final eligibleRange = _layoutDimensionRecoveryRange ?? _visibleGalleryRange;
    if (eligibleRange?.contains(
          queryId: update.queryId,
          revision: update.revision,
          globalItemIndex: update.globalItemIndex,
        ) ??
        false) {
      _pendingLayoutDimensionUpdates[update.globalItemIndex] = update;
      schedulePublication();
    } else {
      _deferredLayoutDimensionUpdates[update.globalItemIndex] = update;
    }
  }

  void updateVisibleRange(LibraryGalleryVisibleRange range) {
    if (_isDisposed ||
        range.queryId != _layoutDimensionQueryId ||
        range.revision != _layoutDimensionRevision) {
      return;
    }
    final previous = _visibleGalleryRange;
    if (previous != null && previous.matches(range)) {
      return;
    }
    _visibleGalleryRange = range;
    if (_layoutDimensionRecoveryRange != null && !_isUserScrolling) {
      return;
    }
    if (_isUserScrolling) {
      _reclassifyLayoutDimensionUpdates(range);
    } else {
      _ensureLayoutDimensionRecoveryEpoch(_readState());
    }
    schedulePublication();
  }

  void _reclassifyLayoutDimensionUpdates(
    LibraryGalleryVisibleRange eligibleRange,
  ) {
    for (final entry in _pendingLayoutDimensionUpdates.entries.toList()) {
      if (!eligibleRange.containsGlobalItemIndex(entry.key)) {
        _pendingLayoutDimensionUpdates.remove(entry.key);
        _deferredLayoutDimensionUpdates[entry.key] = entry.value;
      }
    }
    for (final entry in _deferredLayoutDimensionUpdates.entries.toList()) {
      if (eligibleRange.containsGlobalItemIndex(entry.key)) {
        _deferredLayoutDimensionUpdates.remove(entry.key);
        _pendingLayoutDimensionUpdates[entry.key] = entry.value;
      }
    }
  }

  void _ensureLayoutDimensionRecoveryEpoch(LibraryState state) {
    if (_layoutDimensionRecoveryRange != null) {
      return;
    }
    final visibleRange = _visibleGalleryRange;
    if (visibleRange == null ||
        visibleRange.queryId != state.queryId ||
        visibleRange.revision != state.catalogRevision) {
      return;
    }
    _layoutDimensionRecoveryRange = visibleRange;
    _layoutDimensionRecoveryAnchor = _capturePosition(state);
    _reclassifyLayoutDimensionUpdates(visibleRange);
  }

  void setUserScrolling(bool isScrolling) {
    if (_isDisposed || _isUserScrolling == isScrolling) {
      return;
    }
    _isUserScrolling = isScrolling;
    _onUserScrollingChanged(isScrolling);
    if (isScrolling) {
      _layoutDimensionRecoveryRange = null;
      _layoutDimensionRecoveryAnchor = null;
      _layoutDimensionSettleTimer?.cancel();
      _layoutDimensionSettleTimer = null;
      _layoutDimensionDeadlineTimer?.cancel();
      _layoutDimensionDeadlineTimer = null;
      final visibleRange = _visibleGalleryRange;
      if (visibleRange != null) {
        _reclassifyLayoutDimensionUpdates(visibleRange);
      }
      return;
    }
    final current = _readState();
    _ensureLayoutDimensionRecoveryEpoch(current);
    schedulePublication();
  }

  void schedulePublication() {
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionSettleTimer = null;
    if (_isDisposed ||
        _pendingLayoutDimensionUpdates.isEmpty ||
        _isUserScrolling ||
        _hasPendingTransition()) {
      _layoutDimensionDeadlineTimer?.cancel();
      _layoutDimensionDeadlineTimer = null;
      return;
    }
    _layoutDimensionDeadlineTimer ??= Timer(
      _layoutDimensionMaximumDelay,
      _publishRecoveredDimensions,
    );
    _layoutDimensionSettleTimer = Timer(
      _layoutDimensionQuietDelay,
      _publishRecoveredDimensions,
    );
  }

  void _publishRecoveredDimensions() {
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionSettleTimer = null;
    _layoutDimensionDeadlineTimer?.cancel();
    _layoutDimensionDeadlineTimer = null;
    if (_isDisposed ||
        _pendingLayoutDimensionUpdates.isEmpty ||
        _isUserScrolling ||
        _hasPendingTransition()) {
      return;
    }
    final current = _readState();
    _ensureLayoutDimensionRecoveryEpoch(current);
    final recoveryAnchor = _layoutDimensionRecoveryAnchor;
    final compatible = <int, LibraryGalleryLayoutDimensionUpdate>{};
    for (final entry in _pendingLayoutDimensionUpdates.entries) {
      final update = entry.value;
      if (current.catalogRevision == update.revision &&
          current.queryId == update.queryId) {
        compatible[entry.key] = update;
      }
    }
    _pendingLayoutDimensionUpdates.clear();
    if (compatible.isEmpty) {
      return;
    }
    _publishedLayoutDimensionUpdates.addAll(compatible);
    _dimensionUpdateGeneration += 1;
    _onPublished(recoveryAnchor);
  }

  LibraryGalleryLayoutManifest manifestWithUpdates(
    LibraryGalleryLayoutManifest manifest,
  ) {
    if (!identical(_dimensionUpdateBaseManifest, manifest) ||
        _dimensionUpdatedManifestGeneration != _dimensionUpdateGeneration) {
      _dimensionUpdateBaseManifest = manifest;
      _dimensionUpdatedManifest = manifest.withDimensionUpdates(
        _publishedLayoutDimensionUpdates.values,
      );
      _dimensionUpdatedManifestGeneration = _dimensionUpdateGeneration;
    }
    return _dimensionUpdatedManifest ?? manifest;
  }

  void synchronizeContext(BigInt? revision, String queryId) {
    if (_layoutDimensionRevision == revision &&
        _layoutDimensionQueryId == queryId) {
      return;
    }
    _layoutDimensionRevision = revision;
    _layoutDimensionQueryId = queryId;
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionSettleTimer = null;
    _layoutDimensionDeadlineTimer?.cancel();
    _layoutDimensionDeadlineTimer = null;
    _pendingLayoutDimensionUpdates.clear();
    _deferredLayoutDimensionUpdates.clear();
    _publishedLayoutDimensionUpdates.clear();
    _dimensionUpdateBaseManifest = null;
    _dimensionUpdatedManifest = null;
    _visibleGalleryRange = null;
    _layoutDimensionRecoveryRange = null;
    _layoutDimensionRecoveryAnchor = null;
    _isUserScrolling = false;
    _dimensionUpdateGeneration = 0;
    _dimensionUpdatedManifestGeneration = -1;
  }
}
