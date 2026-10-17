import "dart:async";

import "../application/library_browse_admission.dart";
import "../application/library_query_update.dart";
import "../domain/library_state.dart";

/// Coalesces synchronization revisions without taking catalog publication authority.
class LibrarySynchronizationRefresh {
  LibrarySynchronizationRefresh({
    required this._readState,
    required this._refresh,
    required this._onFailureChanged,
  });

  final LibraryState Function() _readState;
  final Future<LibraryQueryUpdateOutcome> Function(BigInt revision) _refresh;
  final void Function(bool hasFailure) _onFailureChanged;
  BigInt? _pendingRevision;
  Timer? _retryTimer;
  bool _isApplying = false;
  bool _hasFailure = false;
  bool _isDisposed = false;

  void queueRevision(BigInt revision) {
    if (_isDisposed ||
        revision <= (_readState().catalogRevision ?? BigInt.zero)) {
      return;
    }
    _retainPendingRevision(revision);
    _schedule();
  }

  void retry() {
    if (_pendingRevision != null) {
      _schedule();
    }
  }

  void dispose() {
    _isDisposed = true;
    _retryTimer?.cancel();
  }

  void _retainPendingRevision(BigInt revision) {
    final pending = _pendingRevision;
    if (pending == null || revision > pending) {
      _pendingRevision = revision;
    }
  }

  void _schedule() {
    if (_isApplying || _isDisposed) {
      return;
    }
    _retryTimer?.cancel();
    _retryTimer = Timer(Duration.zero, () => unawaited(_drain()));
  }

  Future<void> _drain() async {
    if (_isApplying || _isDisposed) {
      return;
    }
    final target = _pendingRevision;
    if (target == null) {
      return;
    }
    if (!LibraryBrowseAdmission(_readState()).canBrowse) {
      _retryTimer = Timer(const Duration(milliseconds: 250), _schedule);
      return;
    }
    _isApplying = true;
    _pendingRevision = null;
    late final LibraryQueryUpdateOutcome outcome;
    try {
      outcome = await _refresh(target);
    } finally {
      _isApplying = false;
    }
    if (_isDisposed) {
      return;
    }
    final revisionQueuedDuringAttempt = _pendingRevision;
    switch (outcome) {
      case LibraryQueryUpdateOutcome.applied:
        _setFailure(false);
        break;
      case LibraryQueryUpdateOutcome.busy:
      case LibraryQueryUpdateOutcome.superseded:
        _retainPendingRevision(target);
        _retryTimer = Timer(const Duration(milliseconds: 250), _schedule);
        return;
      case LibraryQueryUpdateOutcome.failed:
        _retainPendingRevision(target);
        _setFailure(true);
        if (revisionQueuedDuringAttempt != null) {
          _schedule();
        }
        return;
    }
    if ((_readState().catalogRevision ?? BigInt.zero) < target) {
      _retainPendingRevision(target);
      _setFailure(true);
      if (revisionQueuedDuringAttempt != null) {
        _schedule();
      }
      return;
    }
    if (_pendingRevision != null) {
      _schedule();
    }
  }

  void _setFailure(bool hasFailure) {
    if (_hasFailure == hasFailure) {
      return;
    }
    _hasFailure = hasFailure;
    _onFailureChanged(hasFailure);
  }
}
