import "package:flutter_riverpod/flutter_riverpod.dart";

typedef LibraryScanShutdownAction = Future<void> Function();

class LibraryScanShutdownCoordinator {
  Object? _owner;
  LibraryScanShutdownAction? _action;
  bool _isShuttingDown = false;
  Future<void>? _suspension;

  bool get isShuttingDown => _isShuttingDown;

  void attach(Object owner, LibraryScanShutdownAction action) {
    if (_isShuttingDown) {
      return;
    }
    _owner = owner;
    _action = action;
  }

  void detach(Object owner) {
    if (!identical(_owner, owner)) {
      return;
    }
    _owner = null;
    _action = null;
  }

  Future<void> suspend() {
    _isShuttingDown = true;
    return _suspension ??= _run();
  }

  Future<void> _run() async {
    final action = _action;
    if (action != null) {
      await action();
    }
  }
}

final libraryScanShutdownCoordinatorProvider =
    Provider<LibraryScanShutdownCoordinator>((ref) {
      return LibraryScanShutdownCoordinator();
    });
