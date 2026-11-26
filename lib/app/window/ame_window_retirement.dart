import "dart:async";

import "ame_shutdown_coordinator.dart";

abstract interface class AmeWindowUpdate {
  Future<void> prepare();
  Future<void> commit();
}

class AmeWindowRetirement {
  AmeWindowRetirement({
    required this.shutdown,
    required this.hide,
    required this.destroy,
    required this.onClosing,
    required this.onPreparationFailed,
    this.normalCloseTimeout = const Duration(seconds: 6),
  });

  final AmeShutdownCoordinator shutdown;
  final Future<void> Function() hide;
  final Future<void> Function() destroy;
  final void Function() onClosing;
  final void Function() onPreparationFailed;
  final Duration normalCloseTimeout;
  Future<void>? _operation;

  Future<void> close() => _operation ??= _close();

  Future<void> update(AmeWindowUpdate update) => _operation ??= _update(update);

  Future<void> _close() async {
    onClosing();
    await _hide();
    try {
      await shutdown.shutdown().timeout(normalCloseTimeout);
    } on Object {
      /* Ordinary window closure retains its existing bounded behavior. */
    }
    await destroy();
  }

  Future<void> _update(AmeWindowUpdate update) async {
    onClosing();
    try {
      await update.prepare();
    } on Object {
      _operation = null;
      onPreparationFailed();
      rethrow;
    }
    try {
      await shutdown.shutdown();
      await update.commit();
    } finally {
      // The prepared helper reopens the old version when no commit was written.
      await _hide();
      await destroy();
    }
  }

  Future<void> _hide() async {
    try {
      await hide();
    } on Object {
      /* Retirement continues if the native window cannot be hidden. */
    }
  }
}
