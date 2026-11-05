import "dart:async";

import "package:flutter/foundation.dart";

import "library_exact_analysis_controller.dart";

/// Reconciles gallery membership after native analysis has retired its stream.
class LibraryExactAnalysisPublication extends ChangeNotifier {
  LibraryExactAnalysisPublication(
    this._analysis, {
    required this.refresh,
    required this.canRefresh,
  }) {
    _analysis.addListener(_onAnalysisChanged);
  }

  final LibraryExactAnalysisController _analysis;
  final Future<bool> Function() refresh;
  final bool Function() canRefresh;
  Timer? _retryTimer;
  String? _lastRunId;
  bool _pending = false;
  bool _refreshing = false;
  bool _isDisposed = false;
  bool _hasFailure = false;

  bool get isPublishing {
    final progress = _analysis.progress;
    // Other listeners can observe retirement before this listener queues the refresh.
    final awaitingRetirement =
        !_analysis.isRunning &&
        progress != null &&
        progress.runId != _lastRunId;
    return _pending || _refreshing || awaitingRetirement;
  }

  bool get hasFailure => _hasFailure;

  void retry() {
    if (_isDisposed || _refreshing || !_hasFailure) {
      return;
    }
    _hasFailure = false;
    _pending = true;
    unawaited(_drain());
  }

  void _onAnalysisChanged() {
    final progress = _analysis.progress;
    if (_isDisposed ||
        _analysis.isRunning ||
        progress == null ||
        progress.runId == _lastRunId) {
      return;
    }
    _lastRunId = progress.runId;
    // A zero-read run can publish complete evidence retained by an interrupted
    // process. Refresh after every retirement; the catalog owns error reporting
    // and keeps newer user queries and viewport anchors during publication.
    _pending = true;
    _hasFailure = false;
    if (!_refreshing) {
      unawaited(_drain());
    }
  }

  Future<void> _drain() async {
    _refreshing = true;
    notifyListeners();
    try {
      while (_pending && !_isDisposed) {
        if (!canRefresh()) {
          _retryTimer ??= Timer(const Duration(milliseconds: 250), () {
            _retryTimer = null;
            if (!_isDisposed && !_refreshing && _pending) {
              unawaited(_drain());
            }
          });
          return;
        }
        _pending = false;
        final applied = await refresh();
        if (!_isDisposed) {
          _hasFailure = !applied;
        }
      }
    } finally {
      _refreshing = false;
      if (!_isDisposed) {
        notifyListeners();
      }
    }
  }

  @override
  void dispose() {
    _isDisposed = true;
    _retryTimer?.cancel();
    _pending = false;
    _analysis.removeListener(_onAnalysisChanged);
    super.dispose();
  }
}
