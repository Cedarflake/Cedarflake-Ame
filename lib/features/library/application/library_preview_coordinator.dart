import "dart:async";

import "../domain/library_models.dart";
import "library_preview_failure.dart";
import "library_preview_queue.dart";
import "library_preview_sizing.dart";
import "library_preview_store.dart";
import "library_previewer.dart";

class LibraryPreviewCoordinator {
  factory LibraryPreviewCoordinator({
    required LibraryPreviewer previewer,
    required int defaultPreviewEdge,
    required int maxActive,
    required bool Function(LibraryAsset asset) canPublish,
    required void Function(LibraryAsset asset) onPublished,
  }) {
    return LibraryPreviewCoordinator._(
      previewer,
      defaultPreviewEdge,
      maxActive,
      canPublish,
      onPublished,
    );
  }

  LibraryPreviewCoordinator._(
    LibraryPreviewer previewer,
    this.defaultPreviewEdge,
    int maxActive,
    this._canPublish,
    this._onPublished,
  ) {
    _queue = LibraryPreviewQueue(
      previewer: previewer,
      previewEdge: defaultPreviewEdge,
      maxActive: maxActive,
      onResult: _publish,
      onCompleted: _completed,
      canPublishResult: _canPublish,
    );
  }

  final int defaultPreviewEdge;
  final bool Function(LibraryAsset asset) _canPublish;
  final void Function(LibraryAsset asset) _onPublished;
  final LibraryPreviewStore _store = LibraryPreviewStore();
  late final LibraryPreviewQueue _queue;
  Map<String, _LibraryPreviewDemand> _galleryDemand = const {};
  final LibraryPreviewSizing _sizing = LibraryPreviewSizing();
  LibraryAsset? _viewerDemand;
  bool _isDisposed = false;

  void request(
    LibraryAsset asset, {
    bool retry = false,
    LibraryPreviewPriority priority = LibraryPreviewPriority.visible,
    int? previewEdge,
  }) {
    if (_isDisposed) {
      return;
    }
    final resolved = _store.resolve(asset);
    if (resolved.previewStatus == LibraryPreviewStatus.ready && !retry) {
      return;
    }
    _queue.request(
      resolved,
      retry: retry,
      priority: priority,
      previewEdge: previewEdge ?? defaultPreviewEdge,
    );
  }

  Future<LibraryPreviewRequestOutcome> retry(
    LibraryAsset asset, {
    LibraryPreviewPriority priority = LibraryPreviewPriority.visible,
    int? previewEdge,
  }) {
    if (_isDisposed) {
      return Future.value(LibraryPreviewRequestOutcome.disposed);
    }
    _sizing.allowRetry(asset.locationId);
    return _queue.retry(
      _store.resolve(asset),
      priority: priority,
      previewEdge: previewEdge ?? defaultPreviewEdge,
    );
  }

  LibraryAsset resolve(LibraryAsset asset) => _store.resolve(asset);

  Stream<void> watch(String locationId) => _store.changesFor(locationId);

  void updateGalleryDemand({
    Iterable<LibraryAsset> visible = const <LibraryAsset>[],
    Iterable<LibraryAsset> nearDirection = const <LibraryAsset>[],
    Iterable<LibraryAsset> guard = const <LibraryAsset>[],
    Iterable<LibraryAsset> idle = const <LibraryAsset>[],
    Map<String, int> previewEdges = const <String, int>{},
  }) {
    if (_isDisposed) {
      return;
    }
    final requests = <String, _LibraryPreviewDemand>{};

    void addRequests(
      Iterable<LibraryAsset> assets,
      LibraryPreviewPriority priority,
    ) {
      for (final asset in assets) {
        final current = requests[asset.locationId];
        if (current == null || priority.index > current.priority.index) {
          requests[asset.locationId] = (
            asset: asset,
            priority: priority,
            previewEdge: previewEdges[asset.locationId] ?? defaultPreviewEdge,
          );
        }
      }
    }

    addRequests(idle, LibraryPreviewPriority.idle);
    addRequests(guard, LibraryPreviewPriority.guard);
    addRequests(nearDirection, LibraryPreviewPriority.nearDirection);
    addRequests(visible, LibraryPreviewPriority.visible);
    _galleryDemand = requests;
    // Unchanged visibility can outlive cancelled or rejected work. The queue
    // owns request deduplication against actual pending, active and stored state.
    _applyDemand();
  }

  void updateViewerDemand(LibraryAsset? viewer) {
    if (_isDisposed) {
      return;
    }
    _viewerDemand = viewer;
    _applyDemand();
  }

  void retainPending(Iterable<String> locationIds) {
    if (!_isDisposed) {
      _queue.retainPending(locationIds);
    }
  }

  void updateMaxActive(int maxActive) {
    if (!_isDisposed) {
      _queue.updateMaxActive(maxActive);
    }
  }

  void invalidateAll() {
    if (_isDisposed) {
      return;
    }
    _queue.invalidateAll();
    _store.clear();
    _galleryDemand = const {};
    _sizing.clear();
    _viewerDemand = null;
  }

  void restoreRootAuthority(String rootId) {
    if (_isDisposed) {
      return;
    }
    _queue.clearBlockedRoot(rootId);
    _store.invalidateRoot(rootId);
    _sizing.invalidateRoot(rootId);
  }

  void dispose() {
    if (_isDisposed) {
      return;
    }
    _isDisposed = true;
    _queue.dispose();
    _store.dispose();
    _galleryDemand = const {};
    _sizing.clear();
    _viewerDemand = null;
  }

  void _applyDemand() {
    final requests = {..._galleryDemand};
    final viewer = _viewerDemand;
    if (viewer != null) {
      requests[viewer.locationId] = (
        asset: viewer,
        priority: LibraryPreviewPriority.viewer,
        previewEdge: defaultPreviewEdge,
      );
    }
    _sizing.retainDemand({
      for (final entry in requests.entries)
        entry.key: (
          asset: entry.value.asset,
          previewEdge: entry.value.previewEdge,
        ),
    });
    _store.retain(requests.keys);
    final priorities = {
      for (final MapEntry(key: locationId, value: request) in requests.entries)
        locationId: request.priority,
    };
    if (priorities.isEmpty) {
      _queue.updatePendingDemand(priorities);
      return;
    }
    _queue.replaceDemandAndRequestSizedAll(priorities, [
      for (final priority in LibraryPreviewPriority.values.reversed)
        for (final request in requests.values)
          if (request.priority == priority)
            (
              asset: _store.resolve(request.asset),
              priority: request.priority,
              previewEdge: request.previewEdge,
              ensureSize: _sizing.needsVerification(
                request.asset,
                request.previewEdge,
              ),
            ),
    ]);
  }

  void _publish(LibraryAsset replacement) {
    if (_isDisposed || !_canPublish(replacement)) {
      return;
    }
    _store.publish(replacement);
    if (replacement.previewStatus != LibraryPreviewStatus.ready) {
      _sizing.invalidate(replacement.locationId);
    }
    _onPublished(replacement);
  }

  void _completed(LibraryPreviewCompletion completion) {
    if (_isDisposed || !_canPublish(completion.asset)) {
      return;
    }
    final locationId = completion.asset.locationId;
    final viewer = _viewerDemand;
    final gallery = _galleryDemand[locationId];
    final LibraryPreviewSizeDemand current;
    if (viewer != null && viewer.locationId == locationId) {
      current = (asset: viewer, previewEdge: defaultPreviewEdge);
    } else if (gallery != null) {
      current = (asset: gallery.asset, previewEdge: gallery.previewEdge);
    } else {
      return;
    }
    if (!libraryPreviewSourcesAreCompatible(completion.asset, current.asset)) {
      return;
    }
    final failure = completion.failure;
    if (completion.outcome == LibraryPreviewRequestOutcome.ready) {
      _sizing.recordVerified(completion.asset, completion.previewEdge);
    } else if (completion.outcome == LibraryPreviewRequestOutcome.failed &&
        failure != null &&
        !isLibraryPreviewRootFailure(failure) &&
        completion.asset.previewStatus == LibraryPreviewStatus.ready &&
        completion.previewEdge == current.previewEdge) {
      _sizing.recordFailure(completion.asset, completion.previewEdge);
    }
  }
}

typedef _LibraryPreviewDemand = ({
  LibraryAsset asset,
  LibraryPreviewPriority priority,
  int previewEdge,
});
