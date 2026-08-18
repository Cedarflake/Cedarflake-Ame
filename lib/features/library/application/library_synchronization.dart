import "dart:async";

import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/synchronization.dart" as rust_api;
import "../../../src/rust/domain.dart" as rust_domain;
import "../../../src/rust/domain/library_change.dart" as rust_change;
import "../../../src/rust/domain/library_synchronization.dart" as rust_sync;
import "../domain/library_models.dart";
import "../domain/library_synchronization_models.dart";

abstract interface class LibrarySynchronization {
  LibrarySynchronizationSnapshot get current;

  Stream<LibrarySynchronizationSnapshot> watch();

  Future<void> start();

  Future<void> stop();
}

class InertLibrarySynchronization implements LibrarySynchronization {
  InertLibrarySynchronization();

  final LibrarySynchronizationSnapshot _snapshot =
      LibrarySynchronizationSnapshot.stopped();

  @override
  LibrarySynchronizationSnapshot get current => _snapshot;

  @override
  Future<void> start() async {}

  @override
  Future<void> stop() async {}

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => const Stream.empty();
}

typedef RustSynchronizationCall =
    Future<rust_sync.LibrarySynchronizationSnapshot> Function();
typedef RustSynchronizationStop = Future<void> Function();

class RustLibrarySynchronization implements LibrarySynchronization {
  RustLibrarySynchronization({
    RustSynchronizationCall? startCall,
    RustSynchronizationCall? pollCall,
    RustSynchronizationStop? stopCall,
    this.pollInterval = const Duration(milliseconds: 250),
  }) : _startCall = startCall ?? rust_api.startLibrarySynchronization,
       _pollCall = pollCall ?? rust_api.pollLibrarySynchronization,
       _stopCall = stopCall ?? rust_api.stopLibrarySynchronization;

  final RustSynchronizationCall _startCall;
  final RustSynchronizationCall _pollCall;
  final RustSynchronizationStop _stopCall;
  final Duration pollInterval;
  final StreamController<LibrarySynchronizationSnapshot> _updates =
      StreamController.broadcast(sync: true);
  LibrarySynchronizationSnapshot _current =
      LibrarySynchronizationSnapshot.stopped();
  Timer? _timer;
  Future<void>? _activePoll;
  bool _isStarted = false;
  bool _isStopping = false;
  Future<void>? _stopOperation;

  @override
  LibrarySynchronizationSnapshot get current => _current;

  @override
  Future<void> start() async {
    if (_isStarted || _isStopping) {
      return;
    }
    _isStarted = true;
    await _runCall(_startCall);
    if (_isStopping) {
      return;
    }
    _timer = Timer.periodic(pollInterval, (_) => unawaited(_pollOnce()));
  }

  Future<void> _pollOnce() {
    if (!_isStarted || _isStopping) {
      return Future.value();
    }
    final activePoll = _activePoll;
    if (activePoll != null) {
      return activePoll;
    }
    final operation = _runCall(_pollCall);
    _activePoll = operation;
    return operation.whenComplete(() {
      if (identical(_activePoll, operation)) {
        _activePoll = null;
      }
    });
  }

  Future<void> _runCall(RustSynchronizationCall call) async {
    try {
      _publish(_mapSnapshot(await call()));
    } on Object catch (error) {
      _publish(_current.degraded(_errorCode(error)));
    }
  }

  @override
  Future<void> stop() {
    return _stopOperation ??= _stop();
  }

  Future<void> _stop() async {
    _isStopping = true;
    _timer?.cancel();
    _timer = null;
    await _activePoll;
    if (_isStarted) {
      await _stopCall();
    }
    _isStarted = false;
    _publish(_current.stopped());
  }

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => _updates.stream;

  void _publish(LibrarySynchronizationSnapshot snapshot) {
    if (_current == snapshot) {
      return;
    }
    _current = snapshot;
    if (!_updates.isClosed) {
      _updates.add(snapshot);
    }
  }

  LibrarySynchronizationSnapshot _mapSnapshot(
    rust_sync.LibrarySynchronizationSnapshot snapshot,
  ) {
    return LibrarySynchronizationSnapshot(
      isRunning: snapshot.isRunning,
      catalogRevision: snapshot.catalogRevision,
      appliedMutationCount: snapshot.appliedMutationCount,
      roots: {
        for (final root in snapshot.roots)
          root.rootId: LibraryRootSynchronizationStatus(
            rootId: root.rootId,
            rootGeneration: root.rootGeneration,
            availability: _mapAvailability(root.availability),
            freshness: switch (root.freshness) {
              rust_change.CatalogFreshnessState.synchronized =>
                LibraryCatalogFreshness.synchronized,
              rust_change.CatalogFreshnessState.updating =>
                LibraryCatalogFreshness.updating,
              rust_change.CatalogFreshnessState.needsReconciliation =>
                LibraryCatalogFreshness.needsReconciliation,
              rust_change.CatalogFreshnessState.unavailable =>
                LibraryCatalogFreshness.unavailable,
            },
            freshnessCause: switch (root.freshnessCause) {
              rust_change.CatalogFreshnessCause.noPendingChanges =>
                LibraryCatalogFreshnessCause.noPendingChanges,
              rust_change.CatalogFreshnessCause.pendingChanges =>
                LibraryCatalogFreshnessCause.pendingChanges,
              rust_change.CatalogFreshnessCause.rootUnavailable =>
                LibraryCatalogFreshnessCause.rootUnavailable,
              rust_change.CatalogFreshnessCause.changeSourceUnhealthy =>
                LibraryCatalogFreshnessCause.changeSourceUnhealthy,
              rust_change.CatalogFreshnessCause.evidenceGap =>
                LibraryCatalogFreshnessCause.evidenceGap,
              rust_change.CatalogFreshnessCause.boundedCapacityExceeded =>
                LibraryCatalogFreshnessCause.boundedCapacityExceeded,
            },
            sourceStatus: switch (root.sourceHealth) {
              rust_change.LibraryChangeSourceHealth.healthy =>
                LibraryChangeSourceStatus.healthy,
              rust_change.LibraryChangeSourceHealth.starting =>
                LibraryChangeSourceStatus.starting,
              rust_change.LibraryChangeSourceHealth.degraded =>
                LibraryChangeSourceStatus.degraded,
              rust_change.LibraryChangeSourceHealth.failed =>
                LibraryChangeSourceStatus.failed,
              rust_change.LibraryChangeSourceHealth.stopped =>
                LibraryChangeSourceStatus.stopped,
              rust_change.LibraryChangeSourceHealth.unsupported =>
                LibraryChangeSourceStatus.unsupported,
            },
            pendingChangeCount: root.pendingChangeCount,
            retryWaitCount: root.retryWaitCount,
            freshnessUnknownCount: root.freshnessUnknownCount,
            lastIssueCode: root.lastIssueCode,
          ),
      },
    );
  }

  LibraryRootAvailability _mapAvailability(
    rust_domain.LibraryRootAvailability availability,
  ) {
    return switch (availability) {
      rust_domain.LibraryRootAvailability.unknown =>
        LibraryRootAvailability.unknown,
      rust_domain.LibraryRootAvailability.available =>
        LibraryRootAvailability.available,
      rust_domain.LibraryRootAvailability.missing =>
        LibraryRootAvailability.missing,
      rust_domain.LibraryRootAvailability.inaccessible =>
        LibraryRootAvailability.inaccessible,
      rust_domain.LibraryRootAvailability.offline =>
        LibraryRootAvailability.offline,
    };
  }

  String _errorCode(Object error) {
    if (error case rust_domain.ScanError(:final code)) {
      return code;
    }
    return "library_synchronization_poll_failed";
  }
}

final librarySynchronizationProvider = Provider<LibrarySynchronization>((ref) {
  return InertLibrarySynchronization();
});
