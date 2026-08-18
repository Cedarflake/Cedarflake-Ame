import "dart:async";

typedef AmeShutdownAction = FutureOr<void> Function();

class AmeShutdownCoordinator {
  final List<AmeShutdownAction> _actions = [];
  Future<void>? _shutdown;

  void register(AmeShutdownAction action) {
    if (_shutdown != null) {
      throw StateError("Shutdown has already started");
    }
    _actions.add(action);
  }

  Future<void> shutdown() {
    return _shutdown ??= _run();
  }

  Future<void> _run() async {
    for (final action in _actions.reversed) {
      await action();
    }
  }
}
