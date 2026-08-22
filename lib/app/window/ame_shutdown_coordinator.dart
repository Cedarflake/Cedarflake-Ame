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
    Object? firstFailure;
    StackTrace? firstStackTrace;
    for (final action in _actions.reversed) {
      try {
        await action();
      } on Object catch (error, stackTrace) {
        firstFailure ??= error;
        firstStackTrace ??= stackTrace;
      }
    }
    if (firstFailure != null && firstStackTrace != null) {
      Error.throwWithStackTrace(firstFailure, firstStackTrace);
    }
  }
}
