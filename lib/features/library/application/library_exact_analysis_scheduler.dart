import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_exact_duplicates.dart";
import "../domain/library_models.dart";
import "library_exact_analysis_controller.dart";
import "library_foreground_reads.dart";

/// Schedules published catalog changes after foreground browsing becomes idle.
class LibraryExactAnalysisScheduler extends ChangeNotifier {
  LibraryExactAnalysisScheduler(
    this.analysis, {
    this.idleDelay = const Duration(seconds: 3),
    this._paused = false,
    this.foregroundReads,
  }) {
    analysis.addListener(_analysisChanged);
    foregroundReads?.addListener(_foregroundChanged);
  }

  final LibraryExactAnalysisController analysis;
  final Duration idleDelay;
  final LibraryForegroundReads? foregroundReads;
  BigInt? _revision;
  Map<String, LibraryRootAvailability> _roots = const {};
  Timer? _idleTimer;
  int _requestedGeneration = 0;
  int? _activeGeneration;
  bool _pending = false;
  bool _catalogBusy = true;
  bool _paused;
  bool _closed = false;
  Future<void>? _closing;

  bool get isPaused => _paused;
  bool get isWaiting => _pending && !analysis.isRunning && !_paused;

  void updateCatalog({
    required BigInt? revision,
    required Map<String, LibraryRootAvailability> roots,
    required bool isBusy,
  }) {
    if (_closed) {
      return;
    }
    final changed = revision != _revision || !mapEquals(roots, _roots);
    final busyChanged = _catalogBusy != isBusy;
    _catalogBusy = isBusy;
    if (changed) {
      _revision = revision;
      _roots = Map.unmodifiable(roots);
      _requestedGeneration++;
      _pending = true;
    }
    if (isBusy) {
      _yieldToForeground();
    }
    _schedule();
    if (changed || busyChanged) {
      notifyListeners();
    }
  }

  void setPaused(bool paused) {
    if (_closed || _paused == paused) {
      return;
    }
    _paused = paused;
    if (paused) {
      _yieldToForeground();
    } else {
      _schedule();
    }
    notifyListeners();
  }

  void browsingActivity() {
    if (_closed || analysis.isRunning) {
      return;
    }
    // Input delays admission. Once admitted, actual foreground media leases
    // and explicit tasks own preemption; cached browsing cannot starve analysis.
    _yieldToForeground();
    _schedule();
  }

  void requestNow() {
    if (_closed || _paused) {
      return;
    }
    _requestedGeneration++;
    _pending = true;
    _idleTimer?.cancel();
    _idleTimer = null;
    _schedule(delay: Duration.zero);
    notifyListeners();
  }

  void _yieldToForeground() {
    _idleTimer?.cancel();
    _idleTimer = null;
    if (_activeGeneration != null && analysis.isRunning) {
      unawaited(analysis.stop());
    }
  }

  bool get _canStart =>
      !_closed &&
      !_paused &&
      !_catalogBusy &&
      !(foregroundReads?.isBusy ?? false) &&
      _pending &&
      _revision != null &&
      _roots.isNotEmpty &&
      !analysis.isRunning;

  void _schedule({Duration? delay}) {
    if (!_canStart || _idleTimer != null) {
      return;
    }
    _idleTimer = Timer(delay ?? idleDelay, () {
      _idleTimer = null;
      if (!_canStart) {
        return;
      }
      _activeGeneration = _requestedGeneration;
      analysis.start();
      notifyListeners();
    });
  }

  void _analysisChanged() {
    final completedGeneration = _activeGeneration;
    if (_closed || completedGeneration == null || analysis.isRunning) {
      return;
    }
    _activeGeneration = null;
    // Failures await a new publication or an explicit retry, without an idle retry loop.
    if (analysis.hasFailure ||
        analysis.progress?.phase == LibraryExactAnalysisPhase.completed) {
      if (completedGeneration == _requestedGeneration) {
        _pending = false;
      }
    }
    _schedule();
    notifyListeners();
  }

  void _foregroundChanged() {
    if (foregroundReads?.isBusy ?? false) {
      _yieldToForeground();
    } else {
      _schedule();
    }
  }

  Future<void> close() => _closing ??= _close();

  Future<void> _close() async {
    _closed = true;
    _idleTimer?.cancel();
    _idleTimer = null;
    analysis.removeListener(_analysisChanged);
    foregroundReads?.removeListener(_foregroundChanged);
    await Future.wait<void>([
      analysis.stop(),
      analysis.whenStopped,
    ]).timeout(const Duration(seconds: 10));
  }

  @override
  void dispose() {
    _closed = true;
    _idleTimer?.cancel();
    analysis.removeListener(_analysisChanged);
    foregroundReads?.removeListener(_foregroundChanged);
    super.dispose();
  }
}

// The desktop composition root supplies its native lifecycle-owned instance.
final libraryExactAnalysisSchedulerProvider =
    Provider<LibraryExactAnalysisScheduler?>((ref) => null);
