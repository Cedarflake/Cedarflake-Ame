import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_exact_duplicates.dart";
import "../domain/library_similar_analysis.dart";
import "ports/library_similar_analysis.dart";

class LibrarySimilarAnalysisController extends ChangeNotifier {
  LibrarySimilarAnalysisController(this._service);
  final LibrarySimilarAnalysis _service;
  static int _nextRun = 0;
  _AnalysisRun? _active;
  LibrarySimilarAnalysisProgress? _progress;
  Object? _error;
  bool _suspended = false;
  bool _closed = false;
  bool _disposed = false;
  Future<void>? _closing;

  LibrarySimilarAnalysisProgress? get progress => _progress;
  Object? get error => _error;
  bool get isRunning => _active != null;
  bool get hasFailure =>
      _error != null || _progress?.phase == LibrarySimilarAnalysisPhase.failed;
  Future<void> get whenStopped => _active?.done ?? Future.value();

  void start() {
    if (_closed || _disposed || _active != null) return;
    final run = _AnalysisRun(
      "similar-${DateTime.now().microsecondsSinceEpoch}-${_nextRun++}",
    );
    _active = run;
    _error = null;
    _progress = LibrarySimilarAnalysisProgress(
      runId: run.id,
      phase: LibrarySimilarAnalysisPhase.running,
    );
    run.done = _execute(run);
    notifyListeners();
  }

  Future<void> _execute(_AnalysisRun run) async {
    try {
      final session = await _service.open(run.id);
      run.session = session;
      if (run.cancelled || _closed || _disposed) session.cancel();
      session.suspend(_suspended);
      run.poll = Timer.periodic(const Duration(milliseconds: 200), (_) {
        try {
          _accept(run, session.progress());
        } on Object catch (error) {
          run.controlError = error;
          _cancel(run);
        }
      });
      final progress = await session.run();
      if (run.controlError case final error?) {
        _fail(run, error);
      } else {
        _accept(run, progress);
      }
    } on Object catch (error) {
      _fail(run, error);
    } finally {
      run.poll?.cancel();
      try {
        run.session?.dispose();
      } on Object catch (error) {
        _fail(run, error);
      }
      if (identical(_active, run)) {
        _active = null;
        if (!_disposed) notifyListeners();
      }
    }
  }

  void _accept(_AnalysisRun run, LibrarySimilarAnalysisProgress progress) {
    if (!identical(_active, run) || _disposed) return;
    if (progress.runId != run.id) {
      throw const LibraryDuplicateFailure(
        code: "similarity_run_mismatch",
        message: "Similarity progress belongs to another run",
      );
    }
    _progress = progress;
    notifyListeners();
  }

  void _fail(_AnalysisRun run, Object error) {
    if (!identical(_active, run)) return;
    _error = error;
    final previous = _progress;
    _progress = LibrarySimilarAnalysisProgress(
      runId: run.id,
      phase: LibrarySimilarAnalysisPhase.failed,
      checkedSources: previous?.checkedSources ?? 0,
      analyzedContents: previous?.analyzedContents ?? 0,
      skippedSources: previous?.skippedSources ?? 0,
      issueCode: error is LibraryDuplicateFailure
          ? error.code
          : "similarity_failed",
    );
  }

  void setSuspended(bool suspended) {
    if (_closed || _disposed || _suspended == suspended) return;
    _suspended = suspended;
    final run = _active;
    try {
      run?.session?.suspend(suspended);
    } on Object catch (error) {
      if (run != null) {
        run.controlError = error;
        _cancel(run);
      }
    }
  }

  void _cancel(_AnalysisRun run) {
    run.cancelled = true;
    try {
      run.session?.cancel();
    } on Object catch (error) {
      run.controlError ??= error;
      _fail(run, error);
    }
  }

  Future<void> stop() {
    final run = _active;
    if (run == null) return Future.value();
    _cancel(run);
    return run.done;
  }

  Future<void> close() => _closing ??= _close();
  Future<void> _close() {
    _closed = true;
    return stop();
  }

  @override
  void dispose() {
    _closed = true;
    _disposed = true;
    final run = _active;
    if (run != null) {
      run.poll?.cancel();
      _cancel(run);
    }
    super.dispose();
  }
}

class _AnalysisRun {
  _AnalysisRun(this.id);
  final String id;
  late Future<void> done;
  LibrarySimilarAnalysisSession? session;
  Timer? poll;
  Object? controlError;
  bool cancelled = false;
}

final librarySimilarAnalysisProvider = Provider<LibrarySimilarAnalysis>((ref) {
  throw StateError("Library similarity analysis requires a production binding");
});

final librarySimilarAnalysisControllerProvider =
    Provider<LibrarySimilarAnalysisController>((ref) {
      throw StateError(
        "Library similarity controller requires a lifecycle binding",
      );
    });
