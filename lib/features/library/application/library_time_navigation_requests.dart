import "dart:async";

import "../domain/library_models.dart";
import "../domain/library_time_snapshot.dart";

const _retryDelay = Duration(milliseconds: 120);

class LibraryTimeNavigationRequest {
  LibraryTimeNavigationRequest._({
    required this._generation,
    required this.query,
    required this.timeline,
    required this.anchor,
    required this.globalItemOffset,
  });

  final int _generation;
  final LibraryGalleryQuery query;
  final LibraryTimeline timeline;
  final LibraryTimeAnchor anchor;
  final int globalItemOffset;
  LibraryTimeSnapshot? _resolution;
  LibraryTimeline get currentTimeline => _resolution?.timeline ?? timeline;
  int get currentItemOffset =>
      _resolution?.targetItemOffset ?? globalItemOffset;
  final _completion = Completer<bool>();
}

class LibraryTimeNavigationRequests {
  LibraryTimeNavigationRequests({
    required bool Function() cannotPublish,
    required this._isBlocked,
    required this._isCompatible,
    required this._load,
    required this._onVisibleRangeLoading,
    required this._onTimeAnchorLoadingReleased,
  }) : _hostCannotPublish = cannotPublish;

  final bool Function() _hostCannotPublish;
  final bool Function() _isBlocked;
  final bool Function(LibraryTimeNavigationRequest) _isCompatible;
  final Future<bool> Function(LibraryTimeNavigationRequest) _load;
  final void Function(bool) _onVisibleRangeLoading;
  final void Function() _onTimeAnchorLoadingReleased;
  LibraryTimeNavigationRequest? _pending;
  LibraryTimeNavigationRequest? _active;
  LibraryTimeNavigationRequest? _visibleRangeOwner;
  Timer? _retryTimer;
  int _generation = 0;
  int? _loadingGeneration;
  int? _visibleRangeLoadingGeneration;
  bool _isRunning = false;
  bool _isDisposed = false;

  bool get _cannotPublish => _isDisposed || _hostCannotPublish();
  bool get hasVisibleRangeLoading => _visibleRangeLoadingGeneration != null;
  bool get hasVisibleRangeOwner => _visibleRangeOwner != null;

  bool get _hasCompatibleOwner {
    final owner = _visibleRangeOwner;
    if (owner == null) {
      return false;
    }
    if (_isCompatible(owner)) {
      return true;
    }
    _visibleRangeOwner = null;
    return false;
  }

  Future<bool> request({
    required LibraryGalleryQuery query,
    required LibraryTimeline timeline,
    required LibraryTimeAnchor anchor,
    required int globalItemOffset,
    required bool ownsVisibleRange,
    required bool needsVisibleRangeLoading,
  }) {
    if (_cannotPublish || (!ownsVisibleRange && _hasCompatibleOwner)) {
      return Future.value(false);
    }
    final pending = _pending;
    if (pending != null &&
        _matches(pending, timeline, query, globalItemOffset)) {
      if (ownsVisibleRange) {
        _visibleRangeOwner = pending;
        _setVisibleRangeLoading(pending, needsVisibleRangeLoading);
      }
      return pending._completion.future;
    }
    final active = _active;
    if (active != null &&
        active._generation == _generation &&
        _matches(active, timeline, query, globalItemOffset)) {
      if (ownsVisibleRange) {
        _visibleRangeOwner = active;
        _setVisibleRangeLoading(active, needsVisibleRangeLoading);
      }
      return active._completion.future;
    }
    final request = LibraryTimeNavigationRequest._(
      generation: ++_generation,
      query: query,
      timeline: timeline,
      anchor: anchor,
      globalItemOffset: globalItemOffset,
    );
    _pending = request;
    if (ownsVisibleRange) {
      _visibleRangeOwner = request;
      _setVisibleRangeLoading(request, needsVisibleRangeLoading);
    }
    _settle(pending, false);
    _scheduleDrain();
    return request._completion.future;
  }

  bool accepts(LibraryTimeNavigationRequest request) =>
      !_cannotPublish &&
      request._generation == _generation &&
      _isCompatible(request);

  bool ownsExplicitIntent(LibraryTimeNavigationRequest request) =>
      identical(_visibleRangeOwner, request) && accepts(request);

  void adoptResolution(
    LibraryTimeNavigationRequest request,
    LibraryTimeSnapshot result,
  ) {
    request._resolution = result;
  }

  void beginLoading(LibraryTimeNavigationRequest request) {
    _loadingGeneration = request._generation;
  }

  void releaseLoading(LibraryTimeNavigationRequest request) {
    if (_loadingGeneration != request._generation) {
      return;
    }
    _loadingGeneration = null;
    if (!_cannotPublish) {
      _onTimeAnchorLoadingReleased();
    }
  }

  bool retainsVisibleRange({required int start, required int end}) {
    final owner = _visibleRangeOwner;
    if (owner == null) {
      return true;
    }
    if (!_isCompatible(owner)) {
      _visibleRangeOwner = null;
      return true;
    }
    return owner.currentItemOffset >= start && owner.currentItemOffset < end;
  }

  void retainPassiveInRange({required int start, required int end}) {
    bool contains(LibraryTimeNavigationRequest request) =>
        request.globalItemOffset >= start && request.globalItemOffset < end;

    final pending = _pending;
    if (pending != null && !contains(pending)) {
      if (pending._generation == _generation) {
        _generation += 1;
      }
      _pending = null;
      _settle(pending, false);
    }
    final active = _active;
    if (active != null &&
        !contains(active) &&
        active._generation == _generation) {
      _generation += 1;
    }
    if (_pending == null) {
      _cancelRetry();
    }
  }

  bool cancel() {
    if (_cannotPublish) {
      return false;
    }
    final pending = _pending;
    final active = _active;
    final hasPending = pending != null && !pending._completion.isCompleted;
    final hasActive = active != null && !active._completion.isCompleted;
    if (!hasPending && !hasActive && _visibleRangeOwner == null) {
      return false;
    }
    _generation += 1;
    _retireRequests();
    return true;
  }

  void supersedeQuery() {
    if (_pending != null || _active != null) {
      _generation += 1;
    }
    _retireRequests();
  }

  void dispose() {
    if (_isDisposed) {
      return;
    }
    _isDisposed = true;
    _retireRequests();
    _active = null;
  }

  void _retireRequests() {
    final pending = _pending;
    _pending = null;
    _visibleRangeOwner = null;
    _visibleRangeLoadingGeneration = null;
    _loadingGeneration = null;
    _cancelRetry();
    _settle(pending, false);
    _settle(_active, false);
  }

  void _scheduleDrain() {
    if (_cannotPublish || _isRunning) {
      return;
    }
    _cancelRetry();
    unawaited(Future<void>.microtask(_drain));
  }

  Future<void> _drain() async {
    if (_cannotPublish || _isRunning) {
      return;
    }
    final request = _pending;
    if (request == null) {
      return;
    }
    if (!_isCompatible(request)) {
      _pending = null;
      _releaseVisibleRangeLoading(request);
      if (identical(_visibleRangeOwner, request)) {
        _visibleRangeOwner = null;
      }
      _settle(request, false);
      _scheduleDrain();
      return;
    }
    if (_isBlocked()) {
      _retryTimer ??= Timer(_retryDelay, _scheduleDrain);
      return;
    }
    _pending = null;
    _isRunning = true;
    _active = request;
    try {
      final didLoad = await _load(request);
      final isLatest = request._generation == _generation;
      if (!didLoad && identical(_visibleRangeOwner, request)) {
        _visibleRangeOwner = null;
      }
      _settle(request, didLoad && isLatest);
    } finally {
      _releaseVisibleRangeLoading(request);
      if (identical(_active, request)) {
        _active = null;
      }
      _isRunning = false;
      _scheduleDrain();
    }
  }

  void _setVisibleRangeLoading(
    LibraryTimeNavigationRequest request,
    bool isLoading,
  ) {
    _visibleRangeLoadingGeneration = isLoading ? request._generation : null;
    _onVisibleRangeLoading(isLoading);
  }

  void _releaseVisibleRangeLoading(LibraryTimeNavigationRequest request) {
    if (_visibleRangeLoadingGeneration != request._generation) {
      return;
    }
    _visibleRangeLoadingGeneration = null;
    _onVisibleRangeLoading(false);
  }

  void _cancelRetry() {
    _retryTimer?.cancel();
    _retryTimer = null;
  }

  static void _settle(LibraryTimeNavigationRequest? request, bool value) {
    if (request != null && !request._completion.isCompleted) {
      request._completion.complete(value);
    }
  }

  static bool _matches(
    LibraryTimeNavigationRequest request,
    LibraryTimeline timeline,
    LibraryGalleryQuery query,
    int globalItemOffset,
  ) =>
      request.query == query &&
      request.timeline.revision == timeline.revision &&
      request.timeline.queryId == timeline.queryId &&
      request.globalItemOffset == globalItemOffset;
}
