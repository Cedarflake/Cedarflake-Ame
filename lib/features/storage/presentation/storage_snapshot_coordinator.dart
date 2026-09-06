import "dart:async";

enum StorageSnapshotReadAvailability { available, suspended }

enum StorageSnapshotReadOutcome { settled, superseded }

class StorageSnapshotVersion {
  const StorageSnapshotVersion(this.epoch, this.configurationRevision);

  final int epoch;
  final int configurationRevision;

  @override
  bool operator ==(Object other) =>
      other is StorageSnapshotVersion &&
      other.epoch == epoch &&
      other.configurationRevision == configurationRevision;

  @override
  int get hashCode => Object.hash(epoch, configurationRevision);
}

class StorageSnapshotReadContext {
  const StorageSnapshotReadContext(this.version, this.availability);

  final StorageSnapshotVersion version;
  final StorageSnapshotReadAvailability availability;
}

abstract interface class StorageSnapshotReader {
  StorageSnapshotReadContext get storageSnapshotReadContext;

  Future<StorageSnapshotReadOutcome> readStorageSnapshot(
    StorageSnapshotVersion version,
  );
}

class StorageSnapshotCoordinator {
  StorageSnapshotCoordinator(this._reader);

  final StorageSnapshotReader _reader;
  _StorageSnapshotRefresh? _refresh;
  bool _isDisposed = false;

  bool get isBusy => _refresh != null;

  Future<void> refresh() {
    if (_isDisposed) {
      return Future<void>.value();
    }
    final version = _reader.storageSnapshotReadContext.version;
    final refresh = _refresh ??= _StorageSnapshotRefresh(version);
    refresh.requiredVersion = version;
    _drain(refresh);
    return refresh.completion.future;
  }

  void wake() {
    final refresh = _refresh;
    if (!_isDisposed && refresh != null) {
      refresh.requiredVersion = _reader.storageSnapshotReadContext.version;
      _drain(refresh);
    }
  }

  void _drain(_StorageSnapshotRefresh refresh) {
    if (_isDisposed ||
        !identical(_refresh, refresh) ||
        refresh.activeVersion != null) {
      return;
    }
    final context = _reader.storageSnapshotReadContext;
    if (context.availability == StorageSnapshotReadAvailability.suspended) {
      return;
    }
    final version = context.version;
    refresh.requiredVersion = version;
    refresh.activeVersion = version;
    unawaited(_read(refresh, version));
  }

  Future<void> _read(
    _StorageSnapshotRefresh refresh,
    StorageSnapshotVersion version,
  ) async {
    try {
      final outcome = await _reader.readStorageSnapshot(version);
      if (_isDisposed || !identical(_refresh, refresh)) {
        return;
      }
      refresh.activeVersion = null;
      final context = _reader.storageSnapshotReadContext;
      if (outcome == StorageSnapshotReadOutcome.settled &&
          version == refresh.requiredVersion &&
          version == context.version &&
          context.availability == StorageSnapshotReadAvailability.available) {
        _refresh = null;
        refresh.completion.complete();
      } else {
        _drain(refresh);
      }
    } on Object catch (error, stackTrace) {
      if (!_isDisposed && identical(_refresh, refresh)) {
        _refresh = null;
        refresh.completion.completeError(error, stackTrace);
      }
    }
  }

  void dispose() {
    _isDisposed = true;
    final refresh = _refresh;
    _refresh = null;
    refresh?.completion.complete();
  }
}

class _StorageSnapshotRefresh {
  _StorageSnapshotRefresh(this.requiredVersion);

  StorageSnapshotVersion requiredVersion;
  StorageSnapshotVersion? activeVersion;
  final Completer<void> completion = Completer<void>();
}
