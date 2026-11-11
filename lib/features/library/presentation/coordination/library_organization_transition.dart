import "dart:async";

/// Gives workspace navigation exclusive ownership after the current mode change.
class LibraryOrganizationTransition {
  Completer<void>? _operation;
  bool _navigating = false;
  int _generation = 0;

  bool get isRunning => _operation != null;
  bool get isNavigating => _navigating;
  bool get busy => isRunning || isNavigating;
  int get generation => _generation;
  bool owns(int generation) => generation == _generation;

  Future<void> run(
    Future<void> Function() action, {
    bool supersede = false,
  }) async {
    if (_navigating || (isRunning && !supersede)) return;
    _generation++;
    final completion = Completer<void>();
    _operation = completion;
    try {
      await action();
    } finally {
      if (identical(_operation, completion)) _operation = null;
      completion.complete();
    }
  }

  Future<void> navigate(Future<void> Function() action) async {
    assert(!_navigating);
    _navigating = true;
    try {
      final pending = _operation;
      if (pending != null) await pending.future;
      _generation++;
      await action();
    } finally {
      _navigating = false;
    }
  }
}
