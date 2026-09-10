import "dart:async";

import "package:flutter/foundation.dart";

import "../domain/library_models.dart";
import "library_preview_store.dart";
import "library_previewer.dart";

enum LibraryPreviewPriority { idle, guard, nearDirection, visible, viewer }

enum LibraryPreviewRequestOutcome {
  ready,
  failed,
  updateRequired,
  cancelled,
  superseded,
  contextInvalidated,
  disposed,
}

class LibraryPreviewQueue {
  factory LibraryPreviewQueue({
    required LibraryPreviewer previewer,
    required int previewEdge,
    required int maxActive,
    required void Function(LibraryAsset asset) onResult,
    bool Function(LibraryAsset asset)? canPublishResult,
    Duration rootUnavailableCooldown = const Duration(seconds: 5),
  }) {
    if (maxActive < 1) {
      throw ArgumentError.value(maxActive, "maxActive", "must be positive");
    }
    if (rootUnavailableCooldown.isNegative) {
      throw ArgumentError.value(
        rootUnavailableCooldown,
        "rootUnavailableCooldown",
        "must not be negative",
      );
    }
    return LibraryPreviewQueue._(
      previewer,
      previewEdge,
      maxActive,
      onResult,
      canPublishResult,
      rootUnavailableCooldown,
    );
  }

  LibraryPreviewQueue._(
    this._previewer,
    this._defaultPreviewEdge,
    this._maxActive,
    this._onResult,
    this._canPublishResult,
    this._rootUnavailableCooldown,
  );

  final LibraryPreviewer _previewer;
  final int _defaultPreviewEdge;
  int _maxActive;
  final void Function(LibraryAsset asset) _onResult;
  final bool Function(LibraryAsset asset)? _canPublishResult;
  final Duration _rootUnavailableCooldown;
  final Stopwatch _rootFailureClock = Stopwatch()..start();
  final Map<String, _PreviewRequest> _pending = {};
  final Map<String, _PreviewRequest> _active = {};
  final Map<String, int> _latestGeneration = {};
  final Map<String, _BlockedPreviewRoot> _blockedRoots = {};
  Map<String, LibraryPreviewPriority> _demandPriorities = const {};
  Map<String, int> _demandRanks = const {};
  int _nextSequence = 0;
  int _contextGeneration = 0;
  bool _isDisposed = false;

  @visibleForTesting
  int get debugRetainedGenerationCount => _latestGeneration.length;

  void request(
    LibraryAsset asset, {
    bool retry = false,
    LibraryPreviewPriority priority = LibraryPreviewPriority.visible,
    int? previewEdge,
    bool ensureSize = false,
  }) {
    _enqueue(
      asset,
      retry: retry,
      priority: priority,
      previewEdge: previewEdge,
      ensureSize: ensureSize,
    );
    _drain();
  }

  Future<LibraryPreviewRequestOutcome> retry(
    LibraryAsset asset, {
    LibraryPreviewPriority priority = LibraryPreviewPriority.visible,
    int? previewEdge,
  }) {
    final completion = Completer<LibraryPreviewRequestOutcome>();
    _enqueue(
      asset,
      retry: true,
      priority: priority,
      previewEdge: previewEdge,
      completion: completion,
    );
    _drain();
    return completion.future;
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

  void replaceDemandAndRequestSizedAll(
    Map<String, LibraryPreviewPriority> priorities,
    Iterable<
      ({
        LibraryAsset asset,
        LibraryPreviewPriority priority,
        int previewEdge,
        bool ensureSize,
      })
    >
    requests,
  ) {
    _replacePendingDemand(priorities);
    for (final request in requests) {
      _enqueue(
        request.asset,
        retry: false,
        priority: request.priority,
        previewEdge: request.previewEdge,
        ensureSize: request.ensureSize,
      );
    }
    _drain();
  }

  void _enqueue(
    LibraryAsset asset, {
    required bool retry,
    required LibraryPreviewPriority priority,
    int? previewEdge,
    bool ensureSize = false,
    Completer<LibraryPreviewRequestOutcome>? completion,
  }) {
    final requestedEdge = previewEdge ?? _defaultPreviewEdge;
    if (_isDisposed) {
      completion?.complete(LibraryPreviewRequestOutcome.disposed);
      return;
    }
    final blockedRoot = _blockedRoots[asset.rootId];
    if (blockedRoot != null && blockedRoot.activeScanId != asset.activeScanId) {
      _blockedRoots.remove(asset.rootId);
    } else if (blockedRoot != null &&
        (retry || blockedRoot.hasCooledDown(_rootFailureClock.elapsed))) {
      _blockedRoots.remove(asset.rootId);
    } else if (blockedRoot != null) {
      _publishBlockedAsset(asset, blockedRoot.failure);
      completion?.complete(_rootFailureOutcome(blockedRoot.failure));
      return;
    }
    if (asset.previewStatus == LibraryPreviewStatus.ready &&
        !retry &&
        !ensureSize) {
      return;
    }
    if (asset.previewStatus == LibraryPreviewStatus.failed && !retry) {
      return;
    }

    final source = LibraryPreviewSourceIdentity.fromAsset(asset);
    final pending = _pending[asset.locationId];
    if (pending != null &&
        (!retry || pending.retry) &&
        pending.source == source &&
        pending.previewEdge >= requestedEdge) {
      if (priority.index > pending.priority.index) {
        pending.priority = priority;
      }
      _attachCompletion(pending, completion);
      return;
    }

    final active = _active[asset.locationId];
    if (active != null &&
        (!retry || active.retry) &&
        active.contextGeneration == _contextGeneration &&
        active.source == source &&
        active.previewEdge >= requestedEdge) {
      _attachCompletion(active, completion);
      return;
    }

    if (pending != null) {
      _completeRequest(pending, LibraryPreviewRequestOutcome.superseded);
    }
    if (active != null) {
      _completeRequest(active, LibraryPreviewRequestOutcome.superseded);
    }

    final request = _PreviewRequest(
      asset: asset,
      source: source,
      priority: priority,
      sequence: _nextSequence++,
      generation: (_latestGeneration[asset.locationId] ?? 0) + 1,
      contextGeneration: _contextGeneration,
      retry: retry,
      previewEdge: requestedEdge,
    );
    _attachCompletion(request, completion);
    _latestGeneration[asset.locationId] = request.generation;
    _pending[asset.locationId] = request;
  }

  void cancel(String locationId) {
    final removed = _pending.remove(locationId);
    if (removed != null) {
      _completeRequest(removed, LibraryPreviewRequestOutcome.cancelled);
    }
    _cleanupGeneration(locationId);
  }

  void clearPending() {
    _clearPendingWithOutcome(LibraryPreviewRequestOutcome.cancelled);
  }

  void invalidateAll() {
    _contextGeneration++;
    _blockedRoots.clear();
    _demandPriorities = const {};
    _demandRanks = const {};
    _clearPendingWithOutcome(LibraryPreviewRequestOutcome.contextInvalidated);
    for (final request in _active.values) {
      _completeRequest(
        request,
        LibraryPreviewRequestOutcome.contextInvalidated,
      );
    }
  }

  void clearBlockedRoot(String rootId) {
    if (!_isDisposed) {
      _blockedRoots.remove(rootId);
    }
  }

  void retainPending(Iterable<String> locationIds) {
    final retainedIds = locationIds.toSet();
    final removedIds = <String>[];
    _pending.removeWhere((locationId, request) {
      final shouldRemove =
          !retainedIds.contains(locationId) && !request.hasExplicitWaiter;
      if (shouldRemove) {
        removedIds.add(locationId);
        _completeRequest(request, LibraryPreviewRequestOutcome.cancelled);
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

  void updateMaxActive(int maxActive) {
    if (maxActive < 1) {
      throw ArgumentError.value(maxActive, "maxActive", "must be positive");
    }
    if (_maxActive == maxActive) {
      return;
    }
    _maxActive = maxActive;
    _drain();
  }

  void _replacePendingDemand(Map<String, LibraryPreviewPriority> priorities) {
    _demandPriorities = Map.unmodifiable(priorities);
    var rank = 0;
    _demandRanks = Map.unmodifiable({
      for (final locationId in priorities.keys) locationId: rank++,
    });
    final removedIds = <String>[];
    _pending.removeWhere((locationId, request) {
      final shouldRemove =
          !priorities.containsKey(locationId) && !request.hasExplicitWaiter;
      if (shouldRemove) {
        removedIds.add(locationId);
        _completeRequest(request, LibraryPreviewRequestOutcome.cancelled);
      }
      return shouldRemove;
    });
    for (final request in _pending.values) {
      final priority = priorities[request.asset.locationId];
      if (priority != null) {
        request.priority = priority;
      }
    }
    for (final locationId in removedIds) {
      _cleanupGeneration(locationId);
    }
  }

  void dispose() {
    _isDisposed = true;
    _contextGeneration++;
    _demandPriorities = const {};
    _demandRanks = const {};
    _clearPendingWithOutcome(LibraryPreviewRequestOutcome.disposed);
    for (final request in _active.values) {
      _completeRequest(request, LibraryPreviewRequestOutcome.disposed);
    }
    _latestGeneration.clear();
    _blockedRoots.clear();
  }

  void _drain() {
    while (!_isDisposed) {
      if (_active.length >= _maxActive) {
        return;
      }
      final request = _nextPending();
      if (request == null) {
        return;
      }
      _pending.remove(request.asset.locationId);
      request.activeStartedAt = request.lifetime.elapsed;
      request.isDemandManaged =
          !request.hasExplicitWaiter &&
          _demandPriorities.containsKey(request.asset.locationId);
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
      final requestRank = _demandRanks[request.asset.locationId];
      final currentRank = current == null
          ? null
          : _demandRanks[current.asset.locationId];
      if (current == null ||
          request.priority.index > current.priority.index ||
          (request.priority == current.priority &&
              requestRank != null &&
              (currentRank == null || requestRank < currentRank)) ||
          (request.priority == current.priority &&
              requestRank == currentRank &&
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
        expectedRootId: request.asset.rootId,
        expectedScanId: request.asset.activeScanId,
        expectedSourceRevision: request.asset.sourceRevision,
        expectedSourceGeneration: request.asset.sourceGeneration,
        previewEdge: request.previewEdge,
        force: request.retry,
        protectedLocationIds: {..._demandPriorities.keys, ..._active.keys},
      );
      final rejection = _rejectionOutcome(request, previewed);
      if (rejection != null) {
        _completeRequest(request, rejection);
      } else if (_canPublishResult?.call(previewed) ?? true) {
        _onResult(previewed);
        _completeRequest(request, _completedOutcome(previewed));
      } else {
        _completeRequest(request, LibraryPreviewRequestOutcome.superseded);
      }
    } on Object catch (error) {
      final sourceContextOutcome = _sourceContextFailureOutcome(error);
      if (sourceContextOutcome != null) {
        _completeRequest(
          request,
          _rejectionOutcome(request, request.asset) ?? sourceContextOutcome,
        );
        return;
      }
      if (error is LibraryPreviewFailure && _isRootContextFailure(error)) {
        final rejection = _rejectionOutcome(request, request.asset);
        if (rejection != null) {
          _completeRequest(request, rejection);
        } else {
          _blockRootContext(request, error);
        }
        return;
      }
      final failed = request.asset.withPreview(
        previewPath: request.asset.previewPath,
        width: request.asset.width,
        height: request.asset.height,
        previewStatus: LibraryPreviewStatus.failed,
        previewIssueCode: error is LibraryPreviewFailure
            ? error.code
            : "preview_request_failed",
        previewIssueMessage: error is LibraryPreviewFailure
            ? error.message
            : error.toString(),
      );
      final rejection = _rejectionOutcome(request, failed);
      if (rejection != null) {
        _completeRequest(request, rejection);
      } else if (_canPublishResult?.call(failed) ?? true) {
        _onResult(failed);
        _completeRequest(request, LibraryPreviewRequestOutcome.failed);
      } else {
        _completeRequest(request, LibraryPreviewRequestOutcome.superseded);
      }
    } finally {
      _completeRequest(
        request,
        _rejectionOutcome(request, request.asset) ??
            LibraryPreviewRequestOutcome.failed,
      );
      if (identical(_active[request.asset.locationId], request)) {
        _active.remove(request.asset.locationId);
      }
      _cleanupGeneration(request.asset.locationId);
      _drain();
    }
  }

  void _blockRootContext(
    _PreviewRequest request,
    LibraryPreviewFailure failure,
  ) {
    _blockedRoots[request.asset.rootId] = _BlockedPreviewRoot(
      activeScanId: request.asset.activeScanId,
      failure: failure,
      retryAt: failure.code == "preview_root_unavailable"
          ? _rootFailureClock.elapsed + _rootUnavailableCooldown
          : null,
    );
    _publishBlockedAsset(request.asset, failure);
    final outcome = _rootFailureOutcome(failure);
    _completeRequest(request, outcome);

    final blockedPending = _pending.values
        .where(
          (pending) =>
              pending.asset.rootId == request.asset.rootId &&
              pending.asset.activeScanId == request.asset.activeScanId,
        )
        .toList(growable: false);
    for (final pending in blockedPending) {
      if (!identical(_pending.remove(pending.asset.locationId), pending)) {
        continue;
      }
      _publishBlockedAsset(pending.asset, failure);
      _completeRequest(pending, outcome);
      _cleanupGeneration(pending.asset.locationId);
    }
  }

  void _publishBlockedAsset(LibraryAsset asset, LibraryPreviewFailure failure) {
    if (asset.previewStatus == LibraryPreviewStatus.ready) {
      return;
    }
    if (asset.previewStatus == LibraryPreviewStatus.failed &&
        asset.previewIssueCode == failure.code &&
        asset.previewIssueMessage == failure.message) {
      return;
    }
    final failed = asset.withPreview(
      previewPath: asset.previewPath,
      width: asset.width,
      height: asset.height,
      previewStatus: LibraryPreviewStatus.failed,
      previewIssueCode: failure.code,
      previewIssueMessage: failure.message,
    );
    if (_canPublishResult?.call(failed) ?? true) {
      _onResult(failed);
    }
  }

  LibraryPreviewRequestOutcome? _rejectionOutcome(
    _PreviewRequest request,
    LibraryAsset result,
  ) {
    if (_isDisposed) {
      return LibraryPreviewRequestOutcome.disposed;
    }
    if (request.contextGeneration != _contextGeneration) {
      return LibraryPreviewRequestOutcome.contextInvalidated;
    }
    if (_latestGeneration[request.asset.locationId] != request.generation ||
        !request.source.isCompatibleWith(result)) {
      return LibraryPreviewRequestOutcome.superseded;
    }
    if (request.isDemandManaged &&
        !_demandPriorities.containsKey(request.asset.locationId)) {
      return LibraryPreviewRequestOutcome.cancelled;
    }
    return null;
  }

  void _cleanupGeneration(String locationId) {
    if (!_pending.containsKey(locationId) && !_active.containsKey(locationId)) {
      _latestGeneration.remove(locationId);
    }
  }

  void _attachCompletion(
    _PreviewRequest request,
    Completer<LibraryPreviewRequestOutcome>? completion,
  ) {
    if (completion == null) {
      return;
    }
    request.hasExplicitWaiter = true;
    request.isDemandManaged = false;
    final terminalOutcome = request.terminalOutcome;
    if (terminalOutcome == null) {
      request.completions.add(completion);
    } else {
      completion.complete(terminalOutcome);
    }
  }

  void _completeRequest(
    _PreviewRequest request,
    LibraryPreviewRequestOutcome outcome,
  ) {
    if (request.terminalOutcome != null) {
      return;
    }
    request.terminalOutcome = outcome;
    request.lifetime.stop();
    _logTerminalRequest(request, outcome);
    final completions = request.completions.toList(growable: false);
    request.completions.clear();
    for (final completion in completions) {
      if (!completion.isCompleted) {
        completion.complete(outcome);
      }
    }
  }

  void _clearPendingWithOutcome(LibraryPreviewRequestOutcome outcome) {
    final removed = _pending.values.toList(growable: false);
    _pending.clear();
    for (final request in removed) {
      _completeRequest(request, outcome);
      _cleanupGeneration(request.asset.locationId);
    }
  }

  static LibraryPreviewRequestOutcome _completedOutcome(LibraryAsset asset) {
    return asset.previewStatus == LibraryPreviewStatus.ready
        ? LibraryPreviewRequestOutcome.ready
        : LibraryPreviewRequestOutcome.failed;
  }

  static LibraryPreviewRequestOutcome? _sourceContextFailureOutcome(
    Object error,
  ) {
    if (error is! LibraryPreviewFailure) {
      return null;
    }
    return switch (error.code) {
      "preview_request_superseded" => LibraryPreviewRequestOutcome.superseded,
      "preview_request_context_invalid" =>
        LibraryPreviewRequestOutcome.contextInvalidated,
      _ => null,
    };
  }

  static bool _isRootContextFailure(LibraryPreviewFailure failure) {
    return switch (failure.code) {
      "preview_root_unavailable" ||
      "preview_root_identity_unproven" ||
      "preview_root_identity_changed" ||
      "preview_root_not_found" => true,
      _ => false,
    };
  }

  static LibraryPreviewRequestOutcome _rootFailureOutcome(
    LibraryPreviewFailure failure,
  ) {
    return failure.code == "preview_root_unavailable"
        ? LibraryPreviewRequestOutcome.failed
        : LibraryPreviewRequestOutcome.updateRequired;
  }

  static void _logTerminalRequest(
    _PreviewRequest request,
    LibraryPreviewRequestOutcome outcome,
  ) {
    if (!kDebugMode) {
      return;
    }
    final total = request.lifetime.elapsed;
    final startedAt = request.activeStartedAt;
    final queueWait = startedAt ?? total;
    final active = startedAt == null ? Duration.zero : total - startedAt;
    final slowThreshold = const Duration(milliseconds: 250);
    final isSlowActive = active >= slowThreshold;
    final isSlowViewer =
        request.priority == LibraryPreviewPriority.viewer &&
        total >= slowThreshold;
    final isSlowExplicit = request.hasExplicitWaiter && total >= slowThreshold;
    final isExplicitNonReady =
        request.hasExplicitWaiter &&
        outcome != LibraryPreviewRequestOutcome.ready;
    if (!isSlowActive &&
        !isSlowViewer &&
        !isSlowExplicit &&
        !isExplicitNonReady) {
      return;
    }
    debugPrint(
      "[Ame preview queue] outcome=${outcome.name} retry=${request.retry} "
      "total_ms=${total.inMilliseconds} queue_wait_ms=${queueWait.inMilliseconds} "
      "active_ms=${active.inMilliseconds} location_id=${request.asset.locationId}",
    );
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
    required this.previewEdge,
  });

  final LibraryAsset asset;
  final LibraryPreviewSourceIdentity source;
  LibraryPreviewPriority priority;
  final int sequence;
  final int generation;
  final int contextGeneration;
  final bool retry;
  final int previewEdge;
  bool isDemandManaged = false;
  bool hasExplicitWaiter = false;
  final Stopwatch lifetime = Stopwatch()..start();
  Duration? activeStartedAt;
  LibraryPreviewRequestOutcome? terminalOutcome;
  final List<Completer<LibraryPreviewRequestOutcome>> completions = [];
}

class _BlockedPreviewRoot {
  const _BlockedPreviewRoot({
    required this.activeScanId,
    required this.failure,
    required this.retryAt,
  });

  final String activeScanId;
  final LibraryPreviewFailure failure;
  final Duration? retryAt;

  bool hasCooledDown(Duration now) {
    final retryAt = this.retryAt;
    return retryAt != null && now >= retryAt;
  }
}
