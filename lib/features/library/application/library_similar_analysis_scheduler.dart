import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_models.dart";
import "../domain/library_similar_analysis.dart";
import "library_foreground_reads.dart";
import "library_similar_analysis_controller.dart";

class LibrarySimilarAnalysisScheduler extends ChangeNotifier {
  LibrarySimilarAnalysisScheduler(
    this.analysis, {
    required this.foregroundReads,
  }) {
    analysis.addListener(_analysisChanged);
    foregroundReads.addListener(_foregroundChanged);
  }

  final LibrarySimilarAnalysisController analysis;
  final LibraryForegroundReads foregroundReads;
  static const _idleDelay = Duration(seconds: 3);
  static const _retryDelay = Duration(minutes: 1);
  BigInt? _revision;
  Map<String, LibraryRootAvailability> _roots = const {};
  int _requestedGeneration = 0;
  int? _activeGeneration;
  bool _pending = false;
  bool _catalogBusy = true;
  bool _suspended = true;
  bool _closed = false;
  Object? _environmentError;
  Timer? _timer;
  Timer? _retryTimer;
  Future<void>? _closing;

  Object? get error => _environmentError ?? analysis.error;
  bool get isWaiting => _pending && !analysis.isRunning;

  void updateCatalog({
    required BigInt revision,
    required Map<String, LibraryRootAvailability> roots,
    required bool isBusy,
  }) {
    if (_closed) return;
    final changed = revision != _revision || !mapEquals(roots, _roots);
    final busyChanged = _catalogBusy != isBusy;
    _catalogBusy = isBusy;
    if (changed) {
      _revision = revision;
      _roots = Map.unmodifiable(roots);
      _requestedGeneration++;
      _pending = true;
      _cancelTimer();
    }
    if (changed || busyChanged) {
      _applyForeground();
      notifyListeners();
    }
  }

  void setSuspended(bool suspended) {
    if (_closed || _suspended == suspended) return;
    _suspended = suspended;
    if (suspended) {
      _cancelTimer();
      if (analysis.isRunning) unawaited(analysis.stop());
    }
    _applyForeground();
    notifyListeners();
  }

  void memoryPressure() {
    if (_closed) return;
    _cancelTimer();
    if (analysis.isRunning) unawaited(analysis.stop());
    _pending = true;
    _deferRetry();
  }

  void reportEnvironmentError(Object error, StackTrace stack) {
    if (_closed) return;
    _environmentError = error;
    setSuspended(true);
    notifyListeners();
  }

  void browsingActivity() {
    if (_closed || analysis.isRunning) return;
    _cancelTimer();
    _schedule(_idleDelay);
  }

  bool get _canStart =>
      !_closed &&
      !_suspended &&
      _environmentError == null &&
      !_catalogBusy &&
      !foregroundReads.isBusy &&
      _pending &&
      _retryTimer == null &&
      _revision != null &&
      _roots.values.any(
        (value) =>
            value == LibraryRootAvailability.available ||
            value == LibraryRootAvailability.unknown,
      ) &&
      !analysis.isRunning;

  void _schedule(Duration delay) {
    if (!_canStart || _timer != null) return;
    _timer = Timer(delay, () {
      _timer = null;
      if (!_canStart) return;
      _activeGeneration = _requestedGeneration;
      analysis.start();
      notifyListeners();
    });
  }

  void _applyForeground() {
    final blocked = _suspended || _catalogBusy || foregroundReads.isBusy;
    analysis.setSuspended(blocked);
    if (blocked) {
      _cancelTimer();
    } else {
      _schedule(_idleDelay);
    }
  }

  void _foregroundChanged() {
    if (_closed) return;
    _applyForeground();
    notifyListeners();
  }

  void _analysisChanged() {
    final generation = _activeGeneration;
    if (_closed || generation == null || analysis.isRunning) return;
    _activeGeneration = null;
    final newerPublication = generation != _requestedGeneration;
    final retry =
        analysis.hasFailure || (analysis.progress?.skippedSources ?? 0) > 0;
    final cancelled =
        analysis.progress?.phase == LibrarySimilarAnalysisPhase.cancelled;
    _pending = newerPublication || retry || cancelled;
    if (retry && !newerPublication) _deferRetry();
    _schedule(_idleDelay);
    notifyListeners();
  }

  void _cancelTimer() {
    _timer?.cancel();
    _timer = null;
  }

  void _deferRetry() {
    if (_retryTimer != null) return;
    _retryTimer = Timer(_retryDelay, () {
      _retryTimer = null;
      _schedule(Duration.zero);
    });
  }

  Future<void> close() => _closing ??= _close();
  Future<void> _close() async {
    _closed = true;
    _cancelTimer();
    _retryTimer?.cancel();
    _retryTimer = null;
    analysis.removeListener(_analysisChanged);
    foregroundReads.removeListener(_foregroundChanged);
    await analysis.close();
  }

  @override
  void dispose() {
    _closed = true;
    _cancelTimer();
    _retryTimer?.cancel();
    _retryTimer = null;
    analysis.removeListener(_analysisChanged);
    foregroundReads.removeListener(_foregroundChanged);
    super.dispose();
  }
}

final librarySimilarAnalysisSchedulerProvider =
    Provider<LibrarySimilarAnalysisScheduler?>((ref) {
      throw StateError(
        "Library similarity scheduling requires a lifecycle binding",
      );
    });
