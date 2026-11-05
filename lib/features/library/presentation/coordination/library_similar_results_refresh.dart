import "dart:async";

import "../../application/library_similar_analysis_controller.dart";

class LibrarySimilarResultsRefresh {
  LibrarySimilarResultsRefresh({
    required this.analysis,
    required this.refresh,
  }) {
    analysis.addListener(_schedule);
    _schedule();
  }

  final LibrarySimilarAnalysisController analysis;
  final Future<void> Function() refresh;
  Timer? _timer;
  bool _refreshing = false;
  bool _disposed = false;

  void _schedule() {
    if (_disposed) return;
    if (!analysis.isRunning) {
      _timer?.cancel();
      _timer = null;
    } else if (_timer == null && !_refreshing) {
      _timer = Timer(const Duration(seconds: 2), _refresh);
    }
  }

  Future<void> _refresh() async {
    _timer = null;
    _refreshing = true;
    try {
      await refresh();
    } finally {
      _refreshing = false;
      _schedule();
    }
  }

  void dispose() {
    _disposed = true;
    _timer?.cancel();
    analysis.removeListener(_schedule);
  }
}
