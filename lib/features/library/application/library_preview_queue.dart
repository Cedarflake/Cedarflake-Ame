import "dart:async";

import "../domain/library_models.dart";
import "library_preview_store.dart";
import "library_previewer.dart";

enum LibraryPreviewPriority { idle, guard, nearDirection, visible, viewer }

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
  final Map<String, _PreviewRequest> _pending = {};
  final Map<String, _PreviewRequest> _active = {};
  final Map<String, int> _latestGeneration = {};
  Map<String, LibraryPreviewPriority> _demandPriorities = const {};
  int _nextSequence = 0;
  int _contextGeneration = 0;
  bool _isDisposed = false;

  void request(
    LibraryAsset asset, {
    bool retry = false,
    LibraryPreviewPriority priority = LibraryPreviewPriority.visible,
  }) {
    _enqueue(asset, retry: retry, priority: priority);
    _drain();
  }

  void requestAll(
    Iterable<({LibraryAsset asset, LibraryPreviewPriority priority})> requests,
  ) {
    for (final request in requests) {
      _enqueue(request.asset, retry: false, priority: request.priority);
    }
    _drain();
  }

  void replaceDemandAndRequestAll(
    Map<String, LibraryPreviewPriority> priorities,
    Iterable<({LibraryAsset asset, LibraryPreviewPriority priority})> requests,
  ) {
    _replacePendingDemand(priorities);
    for (final request in requests) {
      _enqueue(request.asset, retry: false, priority: request.priority);
    }
    _drain();
  }

  void _enqueue(
    LibraryAsset asset, {
    required bool retry,
    required LibraryPreviewPriority priority,
  }) {
    if (_isDisposed ||
        (asset.previewStatus == LibraryPreviewStatus.ready && !retry)) {
      return;
    }
    if (asset.previewStatus == LibraryPreviewStatus.failed && !retry) {
      return;
    }

    final source = LibraryPreviewSourceIdentity.fromAsset(asset);
    final pending = _pending[asset.locationId];
    if (pending != null && pending.source == source) {
      if (priority.index > pending.priority.index) {
        pending.priority = priority;
      }
      return;
    }

    final active = _active[asset.locationId];
    if (active != null &&
        active.contextGeneration == _contextGeneration &&
        active.source == source) {
      return;
    }

    final request = _PreviewRequest(
      asset: asset,
      source: source,
      priority: priority,
      sequence: _nextSequence++,
      generation: (_latestGeneration[asset.locationId] ?? 0) + 1,
      contextGeneration: _contextGeneration,
      retry: retry,
    );
    _latestGeneration[asset.locationId] = request.generation;
    _pending[asset.locationId] = request;
  }

  void cancel(String locationId) {
    _pending.remove(locationId);
    _cleanupGeneration(locationId);
  }

  void clearPending() {
    final removedIds = _pending.keys.toList(growable: false);
    _pending.clear();
    for (final locationId in removedIds) {
      _cleanupGeneration(locationId);
    }
  }

  void invalidateAll() {
    _contextGeneration++;
    _demandPriorities = const {};
    clearPending();
  }

  void retainPending(Iterable<String> locationIds) {
    final retainedIds = locationIds.toSet();
    final removedIds = <String>[];
    _pending.removeWhere((locationId, request) {
      final shouldRemove = !retainedIds.contains(locationId);
      if (shouldRemove) {
        removedIds.add(locationId);
      }
      return shouldRemove;
    });
    for (final locationId in removedIds) {
      _cleanupGeneration(locationId);
    }
  }

  void updatePendingDemand(Map<String, LibraryPreviewPriority> priorities) {
    _replacePendingDemand(priorities);
    _drain();
  }

  void _replacePendingDemand(Map<String, LibraryPreviewPriority> priorities) {
    _demandPriorities = Map.unmodifiable(priorities);
    final removedIds = <String>[];
    _pending.removeWhere((locationId, request) {
      final shouldRemove = !priorities.containsKey(locationId);
      if (shouldRemove) {
        removedIds.add(locationId);
      }
      return shouldRemove;
    });
    for (final request in _pending.values) {
      request.priority = priorities[request.asset.locationId]!;
    }
    for (final locationId in removedIds) {
      _cleanupGeneration(locationId);
    }
  }

  void dispose() {
    _isDisposed = true;
    _contextGeneration++;
    _demandPriorities = const {};
    clearPending();
    _latestGeneration.clear();
  }

  void _drain() {
    while (!_isDisposed) {
      final request = _nextPending();
      if (request == null) {
        return;
      }
      final relevantActive = _active.values
          .where(_isActiveDemandRelevant)
          .toList(growable: false);
      final hasBaseCapacity = _active.length < _maxActive;
      final hasObsoleteActive = relevantActive.length < _active.length;
      final activePriorityFloor = relevantActive.fold<LibraryPreviewPriority?>(
        null,
        (lowest, active) {
          final priority = _currentPriority(active);
          if (lowest == null || priority.index < lowest.index) {
            return priority;
          }
          return lowest;
        },
      );
      final mayUsePriorityOverflow =
          _active.length < _maxActive + 1 &&
          (hasObsoleteActive ||
              (activePriorityFloor != null &&
                  request.priority.index > activePriorityFloor.index));
      if (!hasBaseCapacity && !mayUsePriorityOverflow) {
        return;
      }
      _pending.remove(request.asset.locationId);
      request.isDemandManaged = _demandPriorities.containsKey(
        request.asset.locationId,
      );
      _active[request.asset.locationId] = request;
      unawaited(_load(request));
    }
  }

  _PreviewRequest? _nextPending() {
    _PreviewRequest? best;
    for (final request in _pending.values) {
      if (_active.containsKey(request.asset.locationId)) {
        continue;
      }
      final current = best;
      if (current == null ||
          request.priority.index > current.priority.index ||
          (request.priority == current.priority &&
              request.sequence < current.sequence)) {
        best = request;
      }
    }
    return best;
  }

  Future<void> _load(_PreviewRequest request) async {
    try {
      final previewed = await _previewer.materialize(
        locationId: request.asset.locationId,
        previewEdge: _previewEdge,
        retry: request.retry,
        protectedLocationIds: {..._demandPriorities.keys, ..._active.keys},
      );
      if (_canPublish(request, previewed)) {
        _onResult(previewed);
      }
    } on Object catch (error) {
      final failed = request.asset.withPreview(
        previewPath: request.asset.previewPath,
        width: request.asset.width,
        height: request.asset.height,
        previewStatus: LibraryPreviewStatus.failed,
        previewIssueCode: "preview_request_failed",
        previewIssueMessage: error.toString(),
      );
      if (_canPublish(request, failed)) {
        _onResult(failed);
      }
    } finally {
      if (identical(_active[request.asset.locationId], request)) {
        _active.remove(request.asset.locationId);
      }
      _cleanupGeneration(request.asset.locationId);
      _drain();
    }
  }

  bool _canPublish(_PreviewRequest request, LibraryAsset result) {
    return !_isDisposed &&
        request.contextGeneration == _contextGeneration &&
        _latestGeneration[request.asset.locationId] == request.generation &&
        (!request.isDemandManaged ||
            _demandPriorities.containsKey(request.asset.locationId)) &&
        request.source.isCompatibleWith(result);
  }

  bool _isActiveDemandRelevant(_PreviewRequest request) {
    return !request.isDemandManaged ||
        _demandPriorities.containsKey(request.asset.locationId);
  }

  LibraryPreviewPriority _currentPriority(_PreviewRequest request) {
    return request.isDemandManaged
        ? _demandPriorities[request.asset.locationId] ?? request.priority
        : request.priority;
  }

  void _cleanupGeneration(String locationId) {
    if (!_pending.containsKey(locationId) && !_active.containsKey(locationId)) {
      _latestGeneration.remove(locationId);
    }
  }
}

class _PreviewRequest {
  _PreviewRequest({
    required this.asset,
    required this.source,
    required this.priority,
    required this.sequence,
    required this.generation,
    required this.contextGeneration,
    required this.retry,
  });

  final LibraryAsset asset;
  final LibraryPreviewSourceIdentity source;
  LibraryPreviewPriority priority;
  final int sequence;
  final int generation;
  final int contextGeneration;
  final bool retry;
  bool isDemandManaged = false;
}
