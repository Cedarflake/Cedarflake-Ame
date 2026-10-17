import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_exact_duplicates.dart";
import "library_exact_duplicates.dart";

class LibraryExactAnalysisController extends ChangeNotifier {
  LibraryExactAnalysisController(this._service);

  static int _nextRun = 0;

  final LibraryExactDuplicates _service;
  StreamSubscription<LibraryExactAnalysisProgress>? _subscription;
  Completer<void>? _stopped;
  LibraryExactAnalysisProgress? _progress;
  String? _activeRun;
  bool _isStopping = false;
  bool _isDisposed = false;
  bool _hasFailure = false;
  bool _hasStarted = false;

  LibraryExactAnalysisProgress? get progress => _progress;
  bool get isRunning => _activeRun != null;
  bool get isStopping => _isStopping;
  bool get hasFailure => _hasFailure;
  Future<void> get whenStopped => _stopped?.future ?? Future.value();

  void start() {
    if (isRunning || _isDisposed) {
      return;
    }
    final runId =
        "exact-${DateTime.now().microsecondsSinceEpoch}-${_nextRun++}";
    _activeRun = runId;
    _stopped = Completer<void>();
    _isStopping = false;
    _hasFailure = false;
    _hasStarted = false;
    _progress = LibraryExactAnalysisProgress(
      runId: runId,
      phase: LibraryExactAnalysisPhase.running,
      checkedFiles: 0,
      fingerprintedFiles: 0,
      skippedFiles: 0,
    );
    notifyListeners();
    try {
      _subscription = _service
          .analyze(runId)
          .listen(
            (progress) {
              if (_isDisposed ||
                  _activeRun != runId ||
                  progress.runId != runId) {
                return;
              }
              if (!_hasStarted) {
                _hasStarted = true;
                if (_isStopping) {
                  unawaited(_requestCancellation(runId));
                }
              }
              _progress = progress;
              _hasFailure = progress.phase == LibraryExactAnalysisPhase.failed;
              notifyListeners();
            },
            onError: (Object error, StackTrace stack) {
              if (_isDisposed || _activeRun != runId) {
                return;
              }
              _hasFailure = true;
              _finish(runId);
            },
            onDone: () {
              if (_isDisposed || _activeRun != runId) {
                return;
              }
              if (_progress?.phase == LibraryExactAnalysisPhase.running) {
                _hasFailure = true;
              }
              _finish(runId);
            },
          );
    } catch (_) {
      _hasFailure = true;
      _finish(runId);
    }
  }

  Future<void> stop() async {
    final runId = _activeRun;
    if (runId == null || _isStopping || _isDisposed) {
      return;
    }
    _isStopping = true;
    notifyListeners();
    if (!_hasStarted) {
      return;
    }
    await _requestCancellation(runId);
  }

  Future<void> _requestCancellation(String runId) async {
    try {
      await _service.cancel(runId);
    } catch (_) {
      if (_isDisposed || _activeRun != runId) {
        return;
      }
      _isStopping = false;
      _hasFailure = true;
      notifyListeners();
    }
  }

  void _finish(String runId) {
    if (_activeRun != runId) {
      return;
    }
    _activeRun = null;
    _stopped?.complete();
    _isStopping = false;
    final subscription = _subscription;
    _subscription = null;
    unawaited(subscription?.cancel());
    notifyListeners();
  }

  @override
  void dispose() {
    _isDisposed = true;
    final runId = _activeRun;
    if (runId != null) {
      unawaited(_service.cancel(runId).catchError((Object _) {}));
    }
    unawaited(_subscription?.cancel());
    _subscription = null;
    super.dispose();
  }
}

final libraryExactAnalysisControllerProvider =
    Provider<LibraryExactAnalysisController>((ref) {
      final controller = LibraryExactAnalysisController(
        ref.watch(libraryExactDuplicatesProvider),
      );
      ref.onDispose(controller.dispose);
      return controller;
    });
