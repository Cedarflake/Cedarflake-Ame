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
  Future<void>? _recovery;
  Future<void>? _closing;
  final Map<String, BigInt> _pendingRoots = {};
  Map<String, BigInt> _availableRoots = const {};
  BigInt? _catalogRevision;
  bool _importing = true;
  bool _suspended = false;
  bool _isClosing = false;
  Timer? _pressureCooldown;
  Object? _error;

  Object? get error => _error;

  void startInBackground() {
    if (_lifetime != null || _isClosing) {
      return;
    }
    _lifetime = _run();
    unawaited(_lifetime!.catchError(_reportError));
  }

  Future<void> _run() async {
    final session = await _classification.open();
    if (session == null) {
      return;
    }
    try {
      if (_isClosing) {
        session.requestClose();
        return;
      }
      _session = session;
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
