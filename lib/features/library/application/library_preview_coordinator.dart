import "dart:async";

import "../domain/library_models.dart";
import "library_preview_queue.dart";
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
    );
  }

  final int defaultPreviewEdge;
  final bool Function(LibraryAsset asset) _canPublish;
  final void Function(LibraryAsset asset) _onPublished;
  final LibraryPreviewStore _store = LibraryPreviewStore();
  late final LibraryPreviewQueue _queue;
  Map<String, _LibraryPreviewDemand> _galleryDemand = const {};
  final Map<String, ({LibraryPreviewSourceIdentity source, int previewEdge})>
  _verifiedSizes = {};
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
    if (_hasSameGalleryDemand(requests)) {
      return;
    }
    _galleryDemand = requests;
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
    _verifiedSizes.clear();
    _viewerDemand = null;
  }

  void dispose() {
    if (_isDisposed) {
      return;
    }
    _isDisposed = true;
    _queue.dispose();
    _store.dispose();
    _galleryDemand = const {};
    _verifiedSizes.clear();
    _viewerDemand = null;
  }

  bool _hasSameGalleryDemand(Map<String, _LibraryPreviewDemand> next) {
    if (_galleryDemand.length != next.length) {
      return false;
    }
    final currentEntries = _galleryDemand.entries.iterator;
    final nextEntries = next.entries.iterator;
    while (currentEntries.moveNext() && nextEntries.moveNext()) {
      final current = currentEntries.current;
      final candidate = nextEntries.current;
      if (current.key != candidate.key ||
          current.value.priority != candidate.value.priority ||
          current.value.previewEdge != candidate.value.previewEdge ||
          !libraryPreviewSourcesAreCompatible(
            current.value.asset,
            candidate.value.asset,
          )) {
        return false;
      }
    }
    return true;
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
    _verifiedSizes.removeWhere(
      (locationId, _) => !requests.containsKey(locationId),
    );
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
              ensureSize: !_isSizeVerified(request.asset, request.previewEdge),
            ),
    ]);
  }

  bool _isSizeVerified(LibraryAsset asset, int previewEdge) {
    final verified = _verifiedSizes[asset.locationId];
    return verified != null &&
        verified.previewEdge >= previewEdge &&
        verified.source.isCompatibleWith(asset);
  }

  void _publish(LibraryAsset replacement) {
    if (_isDisposed || !_canPublish(replacement)) {
      return;
    }
    _store.publish(replacement);
    if (replacement.previewStatus == LibraryPreviewStatus.ready) {
      var requestedEdge = _galleryDemand[replacement.locationId]?.previewEdge;
      if (_viewerDemand?.locationId == replacement.locationId &&
          (requestedEdge == null || defaultPreviewEdge > requestedEdge)) {
        requestedEdge = defaultPreviewEdge;
      }
      if (requestedEdge != null) {
        _verifiedSizes[replacement.locationId] = (
          source: LibraryPreviewSourceIdentity.fromAsset(replacement),
          previewEdge: requestedEdge,
        );
      }
    } else {
      _verifiedSizes.remove(replacement.locationId);
    }
    _onPublished(replacement);
  }
}

typedef _LibraryPreviewDemand = ({
  LibraryAsset asset,
  LibraryPreviewPriority priority,
  int previewEdge,
});
