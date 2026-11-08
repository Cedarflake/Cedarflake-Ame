import "dart:async";

import "../widgets/library_timeline_projection.dart";

enum LibraryTimeSeekIntent { navigation, prefetch }

class LibraryTimeSeekRequest {
  const LibraryTimeSeekRequest({
    required this.projection,
    required this.value,
    required this.intent,
  });

  final LibraryTimelineProjection projection;
  final double value;
  final LibraryTimeSeekIntent intent;
}

/// Continuous input replaces pending demand without starving an admitted read.
class LibraryTimeSeekScheduler {
  LibraryTimeSeekScheduler(this._run);

  static const _interval = Duration(milliseconds: 120);
  final Future<void> Function(LibraryTimeSeekRequest, int) _run;
  LibraryTimeSeekRequest? _pending;
  int? _activeGeneration;
  int _generation = 0;
  DateTime? _lastStartedAt;
  Timer? _timer;
  bool _isDisposed = false;

  int get generation => _generation;
  bool get isRunning => _activeGeneration != null;
  bool get hasPending => _pending != null;

  bool isCurrent(int generation) => !_isDisposed && generation == _generation;

  void schedule(LibraryTimeSeekRequest request, {bool immediate = false}) {
    if (_isDisposed) return;
    _pending = request;
    if (immediate) {
      _timer?.cancel();
      _timer = null;
    }
    if (!isRunning) _drain(immediate: immediate);
  }

  void clearPending() {
    _timer?.cancel();
    _timer = null;
    _pending = null;
  }

  void cancel() {
    clearPending();
    _generation++;
    _activeGeneration = null;
  }

  void dispose() {
    _isDisposed = true;
    cancel();
  }

  void _drain({bool immediate = false}) {
    if (_isDisposed || isRunning || _pending == null) return;
    final elapsed = _lastStartedAt == null
        ? _interval
        : DateTime.now().difference(_lastStartedAt!);
    if (!immediate && elapsed < _interval) {
      _timer ??= Timer(_interval - elapsed, () {
        _timer = null;
        unawaited(_start());
      });
      return;
    }
    unawaited(_start());
  }

  Future<void> _start() async {
    final request = _pending;
    if (_isDisposed || isRunning || request == null) return;
    _pending = null;
    final generation = ++_generation;
    _activeGeneration = generation;
    _lastStartedAt = DateTime.now();
    try {
      await _run(request, generation);
    } finally {
      if (_activeGeneration == generation) {
        _activeGeneration = null;
        _drain();
      }
    }
  }
}
