import "dart:async";
import "dart:collection";

import "../domain/library_models.dart";
import "library_previewer.dart";

class LibraryPreviewQueue {
  factory LibraryPreviewQueue({
    required LibraryPreviewer previewer,
    required int previewEdge,
    required int maxActive,
    required void Function(LibraryAsset asset) onResult,
  }) {
    return LibraryPreviewQueue._(previewer, previewEdge, maxActive, onResult);
  }

  LibraryPreviewQueue._(
    this._previewer,
    this._previewEdge,
    this._maxActive,
    this._onResult,
  );

  final LibraryPreviewer _previewer;
  final int _previewEdge;
  final int _maxActive;
  final void Function(LibraryAsset asset) _onResult;
  final Queue<LibraryAsset> _pending = Queue();
  final Set<String> _pendingIds = {};
  final Set<String> _activeIds = {};
  bool _isDisposed = false;

  void request(LibraryAsset asset, {bool retry = false}) {
    if (_isDisposed || asset.previewStatus == LibraryPreviewStatus.ready) {
      return;
    }
    if (asset.previewStatus == LibraryPreviewStatus.failed && !retry) {
      return;
    }
    if (_pendingIds.contains(asset.locationId) ||
        _activeIds.contains(asset.locationId)) {
      return;
    }
    _pending.addLast(asset);
    _pendingIds.add(asset.locationId);
    _drain();
  }

  void cancel(String locationId) {
    if (!_pendingIds.remove(locationId)) {
      return;
    }
    _pending.removeWhere((asset) => asset.locationId == locationId);
  }

  void clearPending() {
    _pending.clear();
    _pendingIds.clear();
  }

  void dispose() {
    _isDisposed = true;
    clearPending();
  }

  void _drain() {
    while (!_isDisposed &&
        _activeIds.length < _maxActive &&
        _pending.isNotEmpty) {
      final asset = _pending.removeFirst();
      _pendingIds.remove(asset.locationId);
      _activeIds.add(asset.locationId);
      unawaited(_load(asset));
    }
  }

  Future<void> _load(LibraryAsset asset) async {
    try {
      final previewed = await _previewer.materialize(
        locationId: asset.locationId,
        previewEdge: _previewEdge,
      );
      if (!_isDisposed) {
        _onResult(previewed);
      }
    } on Object catch (error) {
      if (!_isDisposed) {
        _onResult(
          asset.withPreview(
            previewPath: asset.previewPath,
            width: asset.width,
            height: asset.height,
            previewStatus: LibraryPreviewStatus.failed,
            previewIssueCode: "preview_request_failed",
            previewIssueMessage: error.toString(),
          ),
        );
      }
    } finally {
      _activeIds.remove(asset.locationId);
      _drain();
    }
  }
}
