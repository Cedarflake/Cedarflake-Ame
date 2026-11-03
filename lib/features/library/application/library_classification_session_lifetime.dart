import "dart:async";

import "library_classification_environment.dart";
import "ports/library_classification.dart";

class LibraryClassificationSessionLifetime {
  LibraryClassificationSessionLifetime(
    this._classification,
    this._environment,
    this._reportError,
    this._reportOpened,
  );

  final LibraryClassification _classification;
  final LibraryClassificationEnvironment _environment;
  final void Function(Object, StackTrace) _reportError;
  final void Function() _reportOpened;
  LibraryClassificationSession? _session;
  LibraryClassificationMaintenance? _maintenance;
  Future<void>? _lifetime;
  Future<LibraryClassificationSession?>? _opening;
  Future<void>? _closing;
  Future<void>? _configurationChange;
  bool _isClosing = false;
  bool _isChangingConfiguration = false;

  Future<void> waitUntilOpened() async {
    await _opening;
  }

  void startInBackground() {
    if (_lifetime != null || _isClosing || _isChangingConfiguration) {
      return;
    }
    _lifetime = _run();
    unawaited(_lifetime!.catchError(_reportError));
  }

  Future<void> _run() async {
    while (!_isClosing && !_isChangingConfiguration) {
      _opening = _classification.open();
      final session = await _opening;
      if (session == null) {
        _reportOpened();
        return;
      }
      LibraryClassificationMaintenance? plan;
      try {
        try {
          if (_isClosing || _isChangingConfiguration) {
            return;
          }
          _session = session;
          _reportOpened();
          _environment.attach(session);
          plan = await session.run();
          _maintenance = plan;
        } finally {
          _session = null;
          try {
            _environment.requestRetirement(session);
          } finally {
            try {
              await _environment.joinRecovery();
            } finally {
              await session.close();
            }
          }
        }
        if (plan == null) {
          return;
        }
        if (_isClosing || _isChangingConfiguration) {
          plan.cancel();
          return;
        }
        await plan.commit();
      } finally {
        _maintenance = null;
        plan?.dispose();
      }
    }
  }

  void _requestRetirement() {
    _maintenance?.cancel();
    final session = _session;
    if (session != null) {
      _environment.requestRetirement(session);
    }
  }

  Future<T> changeConfiguration<T>(Future<T> Function() change) {
    if (_isClosing ||
        _configurationChange != null ||
        _isChangingConfiguration) {
      return Future.error(
        StateError("Classification configuration is unavailable"),
      );
    }
    _isChangingConfiguration = true;
    final operation =
        Future<T>.sync(() async {
          _requestRetirement();
          try {
            await _lifetime;
          } on Object catch (error, stackTrace) {
            // Native admission separately proves retirement before accepting configuration.
            _reportError(error, stackTrace);
          }
          if (_isClosing) {
            throw StateError("Classification is closing");
          }
          return change();
        }).whenComplete(() async {
          _lifetime = null;
          _isChangingConfiguration = false;
          startInBackground();
          try {
            await waitUntilOpened();
          } finally {
            _configurationChange = null;
          }
        });
    _configurationChange = operation.then<void>(
      (_) {},
      onError: (Object _, StackTrace _) {},
    );
    return operation;
  }

  Future<void> close() {
    if (_closing != null) {
      return _closing!;
    }
    _isClosing = true;
    _environment.stop();
    _closing = Future<void>.sync(() async {
      _requestRetirement();
      await _configurationChange;
      await _lifetime;
    });
    return _closing!;
  }
}
