import "dart:async";

import "library_classification.dart";
import "library_foreground_reads.dart";

class LibraryClassificationEnvironment {
  LibraryClassificationEnvironment(this._foregroundReads, this._reportError) {
    _foregroundReads.addListener(_foregroundChanged);
  }

  final LibraryForegroundReads _foregroundReads;
  final void Function(Object, StackTrace) _reportError;
  LibraryClassificationSession? _session;
  Future<void>? _recovery;
  final Map<String, BigInt> _pendingRoots = {};
  Map<String, BigInt> _availableRoots = const {};
  BigInt? _catalogRevision;
  bool _importing = true;
  bool _suspended = false;
  bool _isClosed = false;
  Timer? _pressureCooldown;

  void attach(LibraryClassificationSession session) {
    _session = session;
    _pendingRoots.addAll(_availableRoots);
    _foregroundChanged();
    session.catalogChanged();
    session.browsingActivity();
    _recoverRoots();
  }

  void requestRetirement(LibraryClassificationSession session) {
    if (identical(_session, session)) {
      _session = null;
    }
    session.requestClose();
  }

  Future<void> joinRecovery() async {
    await _recovery;
  }

  void updateCatalog({
    required BigInt revision,
    required Map<String, BigInt> availableRoots,
    required bool isBusy,
  }) {
    if (_isClosed) {
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
    if (_isClosed || suspended == _suspended) {
      return;
    }
    _suspended = suspended;
    _foregroundChanged();
  }

  void memoryPressure() {
    if (_isClosed) {
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
    if (_isClosed || session == null) {
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
    if (_isClosed ||
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
    while (!_isClosed &&
        identical(_session, session) &&
        _pendingRoots.isNotEmpty) {
      final root = _pendingRoots.entries.first;
      _pendingRoots.remove(root.key);
      try {
        await session.rootAvailable(root.key, root.value);
      } on Object catch (error, stackTrace) {
        if (!_isClosed && _availableRoots[root.key] == root.value) {
          _reportError(error, stackTrace);
        }
      }
    }
  }

  void stop() {
    _isClosed = true;
    _pressureCooldown?.cancel();
    _foregroundReads.removeListener(_foregroundChanged);
    _pendingRoots.clear();
  }
}
