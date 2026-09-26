import "package:flutter_riverpod/flutter_riverpod.dart";

typedef LibraryScanShutdownAction = Future<void> Function();

class LibraryScanShutdownCoordinator {
  final Map<Object, LibraryScanShutdownAction> _actions = {};
  bool _isShuttingDown = false;
  Future<void>? _suspension;

  bool get isShuttingDown => _isShuttingDown;

  void attach(Object owner, LibraryScanShutdownAction action) {
    if (_isShuttingDown) {
      return;
    }
    _actions[owner] = action;
  }

  void detach(Object owner) {
    _actions.remove(owner);
  }

  Future<void> suspend() {
    _isShuttingDown = true;
    return _suspension ??= _run();
  }

  Future<void> _run() async {
    final actions = _actions.values.toList(growable: false);
    await Future.wait(actions.map((action) => action()));
  }
}

final libraryScanShutdownCoordinatorProvider =
    Provider<LibraryScanShutdownCoordinator>((ref) {
      return LibraryScanShutdownCoordinator();
    });
