import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "library_classification.dart";
import "library_foreground_reads.dart";

final libraryClassificationLifecycleProvider =
    Provider<LibraryClassificationLifecycle?>((ref) => null);

class LibraryClassificationLifecycle extends ChangeNotifier {
  LibraryClassificationLifecycle(
    this._classification, {
    required this._foregroundReads,
  }) {
    _foregroundReads.addListener(_foregroundChanged);
  }

  final LibraryClassification _classification;
  final LibraryForegroundReads _foregroundReads;
  LibraryClassificationSession? _session;
  Future<void>? _lifetime;
  Future<LibraryClassificationSession?>? _opening;
  Future<void>? _recovery;
  Future<void>? _closing;
  Future<void>? _configurationChange;
  final Map<String, BigInt> _pendingRoots = {};
  Map<String, BigInt> _availableRoots = const {};
  BigInt? _catalogRevision;
  bool _importing = true;
  bool _suspended = false;
  bool _isClosing = false;
  bool _isChangingConfiguration = false;
  Timer? _pressureCooldown;
  Object? _error;

  Object? get error => _error;

  void reportEnvironmentError(Object error, StackTrace stackTrace) =>
      _reportError(error, stackTrace);

  Future<void> waitUntilOpened() async {
    try {
      await _opening;
    } on Object catch (error, stackTrace) {
      _reportError(error, stackTrace);
    }
  }

  void startInBackground() {
    if (_lifetime != null || _isClosing || _isChangingConfiguration) {
      return;
    }
    _lifetime = _run();
    unawaited(_lifetime!.catchError(_reportError));
  }

  Future<void> _run() async {
    _opening = _classification.open();
    final session = await _opening;
    if (session == null) {
      return;
    }
    try {
      if (_isClosing || _isChangingConfiguration) {
        session.requestClose();
        return;
      }
      _session = session;
      _pendingRoots.addAll(_availableRoots);
      _foregroundChanged();
      session.catalogChanged();
      session.browsingActivity();
      _recoverRoots();
      await session.run();
    } finally {
      _session = null;
      session.requestClose();
      try {
        await _recovery;
      } finally {
        await session.close();
      }
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
        Future<T>.microtask(() async {
          _session?.requestClose();
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

  void updateCatalog({
    required BigInt revision,
    required Map<String, BigInt> availableRoots,
    required bool isBusy,
  }) {
    if (_isClosing) {
      return;
    }
    if (_importing != isBusy) {
      _importing = isBusy;
      _foregroundChanged();
    }
    if (_catalogRevision != revision) {
      _catalogRevision = revision;
      _signal((session) => session.catalogChanged());
    }
    _pendingRoots.removeWhere((id, _) => !availableRoots.containsKey(id));
    for (final entry in availableRoots.entries) {
      if (_availableRoots[entry.key] != entry.value) {
        _pendingRoots[entry.key] = entry.value;
      }
    }
    _availableRoots = Map.of(availableRoots);
    _recoverRoots();
  }

  void setSuspended(bool suspended) {
    if (_isClosing || suspended == _suspended) {
      return;
    }
    _suspended = suspended;
    _foregroundChanged();
  }

  void memoryPressure() {
    if (_isClosing) {
      return;
    }
    _pressureCooldown?.cancel();
    _pressureCooldown = Timer(const Duration(minutes: 1), () {
      _pressureCooldown = null;
      _foregroundChanged();
    });
    _foregroundChanged();
  }

  void browsingActivity() => _signal((session) => session.browsingActivity());

  void _foregroundChanged() => _signal((session) {
    session.foregroundChanged(
      readingSources: _foregroundReads.isBusy,
      importing: _importing,
      suspended: _suspended,
      memoryPressure: _pressureCooldown != null,
    );
  });

  void _signal(void Function(LibraryClassificationSession) action) {
    final session = _session;
    if (_isClosing || session == null) {
      return;
    }
    try {
      action(session);
    } on Object catch (error, stackTrace) {
      _reportError(error, stackTrace);
    }
  }

  void _recoverRoots() {
    final session = _session;
    if (_isClosing ||
        _isChangingConfiguration ||
        session == null ||
        _recovery != null ||
        _pendingRoots.isEmpty) {
      return;
    }
    _recovery = _drainRoots(session).whenComplete(() {
      _recovery = null;
      _recoverRoots();
    });
    unawaited(_recovery!.catchError(_reportError));
  }

  Future<void> _drainRoots(LibraryClassificationSession session) async {
    while (!_isClosing &&
        !_isChangingConfiguration &&
        identical(_session, session) &&
        _pendingRoots.isNotEmpty) {
      final root = _pendingRoots.entries.first;
      _pendingRoots.remove(root.key);
      try {
        await session.rootAvailable(root.key, root.value);
      } on Object catch (error, stackTrace) {
        if (!_isClosing && _availableRoots[root.key] == root.value) {
          _reportError(error, stackTrace);
        }
      }
    }
  }

  Future<void> close() {
    if (_closing != null) {
      return _closing!;
    }
    _isClosing = true;
    _pressureCooldown?.cancel();
    _foregroundReads.removeListener(_foregroundChanged);
    _pendingRoots.clear();
    _closing = Future<void>.sync(() async {
      _session?.requestClose();
      await _configurationChange;
      await _lifetime;
    });
    return _closing!;
  }

  void _reportError(Object error, StackTrace stackTrace) {
    _error = error;
    debugPrint("[Ame classification] lifecycle failed: ${error.runtimeType}");
    notifyListeners();
  }
}
