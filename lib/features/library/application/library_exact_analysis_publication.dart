import "dart:async";

import "library_exact_analysis_controller.dart";

/// Reconciles gallery membership after native analysis has retired its stream.
class LibraryExactAnalysisPublication {
  LibraryExactAnalysisPublication(this._analysis, {required this.refresh}) {
    _analysis.addListener(_onAnalysisChanged);
  }

  final LibraryExactAnalysisController _analysis;
  final Future<bool> Function() refresh;
  String? _lastRunId;
  bool _pending = false;
  bool _refreshing = false;
  bool _isDisposed = false;

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
    if (!_refreshing) {
      unawaited(_drain());
    }
  }

  Future<void> _drain() async {
    _refreshing = true;
    try {
      while (_pending && !_isDisposed) {
        _pending = false;
        await refresh();
      }
    } finally {
      _refreshing = false;
    }
  }

  void dispose() {
    _isDisposed = true;
    _pending = false;
    _analysis.removeListener(_onAnalysisChanged);
  }
}
