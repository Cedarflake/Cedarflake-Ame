import "dart:async";

/// Gives workspace navigation exclusive ownership after the current mode change.
class LibraryOrganizationTransition {
  Completer<void>? _operation;
  bool _navigating = false;

  bool get isRunning => _operation != null;
  bool get isNavigating => _navigating;
  bool get busy => isRunning || isNavigating;

  Future<void> run(Future<void> Function() action) async {
    if (busy) return;
    final completion = Completer<void>();
    _operation = completion;
    try {
      await action();
    } finally {
      _operation = null;
      completion.complete();
    }
  }

  Future<void> navigate(Future<void> Function() action) async {
    assert(!_navigating);
    _navigating = true;
    try {
      final pending = _operation;
      if (pending != null) await pending.future;
      await action();
    } finally {
      _navigating = false;
    }
  }
}
