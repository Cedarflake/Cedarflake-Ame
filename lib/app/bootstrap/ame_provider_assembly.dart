import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_riverpod/misc.dart";

import "../window/ame_shutdown_coordinator.dart";

class AmeProviderAssembly {
  AmeProviderAssembly({
    required AmeShutdownCoordinator shutdown,
    this._unmountApplication,
  }) {
    shutdown.register(_close);
  }

  final Future<void> Function()? _unmountApplication;
  ProviderContainer? _container;
  bool _isClosed = false;

  ProviderContainer assemble({
    required List<Override> overrides,
    required List<ProviderListenable<Object?>> requirements,
  }) {
    if (_isClosed || _container != null) {
      throw StateError(
        "The application Provider container has already been assembled or closed",
      );
    }
    final container = ProviderContainer(overrides: overrides);
    try {
      for (final provider in requirements) {
        container.read(provider);
      }
      _container = container;
      return container;
    } on Object {
      container.dispose();
      rethrow;
    }
  }

  Future<void> _close() async {
    if (_isClosed) return;
    _isClosed = true;
    await _unmountApplication?.call();
    _container?.dispose();
    _container = null;
  }
}

Future<T> acquireAmeApplication<T>({
  required AmeShutdownCoordinator shutdown,
  required Future<T> Function() acquire,
}) async {
  try {
    return await acquire();
  } on Object catch (error, stackTrace) {
    try {
      await shutdown.shutdown();
    } on Object catch (cleanupError) {
      Error.throwWithStackTrace(
        AmeBootstrapCleanupFailure(error, cleanupError),
        stackTrace,
      );
    }
    Error.throwWithStackTrace(error, stackTrace);
  }
}

class AmeBootstrapCleanupFailure implements Exception {
  const AmeBootstrapCleanupFailure(this.cause, this.cleanupError);

  final Object cause;
  final Object cleanupError;

  @override
  String toString() => "$cause\nResource cleanup failed: $cleanupError";
}
