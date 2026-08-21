import "dart:async";

import "package:flutter/foundation.dart";
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
typedef SynchronizationClock = DateTime Function();

class RustLibrarySynchronization implements LibrarySynchronization {
  RustLibrarySynchronization({
    RustSynchronizationCall? startCall,
    RustSynchronizationCall? pollCall,
    RustSynchronizationStop? stopCall,
    this.pollInterval = const Duration(milliseconds: 250),
    this.transientFailureTolerance = const Duration(seconds: 30),
    SynchronizationClock? now,
    this.enableDebugLogging = kDebugMode,
  }) : _startCall = startCall ?? rust_api.startLibrarySynchronization,
       _pollCall = pollCall ?? rust_api.pollLibrarySynchronization,
       _stopCall = stopCall ?? rust_api.stopLibrarySynchronization,
       _now = now ?? DateTime.now;

  final RustSynchronizationCall _startCall;
  final RustSynchronizationCall _pollCall;
  final RustSynchronizationStop _stopCall;
  final Duration pollInterval;
  final Duration transientFailureTolerance;
  final SynchronizationClock _now;
  final bool enableDebugLogging;
  final StreamController<LibrarySynchronizationSnapshot> _updates =
      StreamController.broadcast(sync: true);
  LibrarySynchronizationSnapshot _current =
      LibrarySynchronizationSnapshot.stopped();
  Timer? _timer;
  Future<void>? _activePoll;
  bool _isStarted = false;
  bool _isStopping = false;
  Future<void>? _stopOperation;
  DateTime? _transientFailureStartedAt;
  String? _transientFailureCode;
  int _transientFailureCount = 0;
  DateTime? _lastDebugSnapshotAt;
  Map<String, String> _lastDebugRootSignatures = const {};

  @override
  LibrarySynchronizationSnapshot get current => _current;

  @override
  Future<void> start() async {
    if (_isStarted || _isStopping) {
      return;
    }
    _isStarted = true;
    await _runCall(_startCall, phase: "start");
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
    final operation = _runCall(_pollCall, phase: "poll");
    _activePoll = operation;
    return operation.whenComplete(() {
      if (identical(_activePoll, operation)) {
        _activePoll = null;
      }
    });
  }

  Future<void> _runCall(
    RustSynchronizationCall call, {
    required String phase,
  }) async {
    final stopwatch = Stopwatch()..start();
    try {
      final snapshot = _retainUnresolvedFailures(_mapSnapshot(await call()));
      _clearTransientFailure();
      _publish(snapshot, phase: phase, elapsed: stopwatch.elapsed);
    } on Object catch (error) {
      final errorCode = _errorCode(error);
      if (_shouldRetryTransiently(errorCode)) {
        _debugFailure(
          phase: phase,
          errorCode: errorCode,
          error: error,
          elapsed: stopwatch.elapsed,
          isTransient: true,
        );
        return;
      }
      _debugFailure(
        phase: phase,
        errorCode: errorCode,
        error: error,
        elapsed: stopwatch.elapsed,
        isTransient: false,
      );
      _clearTransientFailure();
      _publish(
        _current.degraded(errorCode),
        phase: phase,
        elapsed: stopwatch.elapsed,
      );
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
    _publish(_current.stopped(), phase: "stop", elapsed: Duration.zero);
  }

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => _updates.stream;

  void _publish(
    LibrarySynchronizationSnapshot snapshot, {
    required String phase,
    required Duration elapsed,
  }) {
    if (_current == snapshot) {
      _debugSnapshot(phase, snapshot, elapsed);
      return;
    }
    _current = snapshot;
    _debugSnapshot(phase, snapshot, elapsed);
    if (!_updates.isClosed) {
      _updates.add(snapshot);
    }
  }

  LibrarySynchronizationSnapshot _retainUnresolvedFailures(
    LibrarySynchronizationSnapshot incoming,
  ) {
    var retainedFailure = false;
    final roots = <String, LibraryRootSynchronizationStatus>{};
    for (final entry in incoming.roots.entries) {
      final current = entry.value;
      final previous = _current.roots[entry.key];
      if (previous?.freshness == LibraryCatalogFreshness.needsReconciliation &&
          current.freshness == LibraryCatalogFreshness.updating) {
        retainedFailure = true;
        roots[entry.key] = LibraryRootSynchronizationStatus(
          rootId: current.rootId,
          rootGeneration: current.rootGeneration,
          availability: current.availability,
          freshness: LibraryCatalogFreshness.needsReconciliation,
          freshnessCause: previous!.freshnessCause,
          sourceStatus: current.sourceStatus,
          pendingChangeCount: current.pendingChangeCount,
          retryWaitCount: current.retryWaitCount,
          freshnessUnknownCount: current.freshnessUnknownCount,
          lastIssueCode: previous.lastIssueCode,
        );
      } else {
        roots[entry.key] = current;
      }
    }
    if (!retainedFailure) {
      return incoming;
    }
    return LibrarySynchronizationSnapshot(
      isRunning: incoming.isRunning,
      catalogRevision: incoming.catalogRevision,
      appliedMutationCount: incoming.appliedMutationCount,
      roots: roots,
      lastErrorCode: _current.lastErrorCode ?? incoming.lastErrorCode,
    );
  }

  bool _shouldRetryTransiently(String errorCode) {
    if (errorCode != "catalog_database_busy" &&
        errorCode != "catalog_database_locked") {
      return false;
    }
    final now = _now();
    if (_transientFailureCode != errorCode) {
      _transientFailureCode = errorCode;
      _transientFailureStartedAt = now;
      _transientFailureCount = 1;
    } else {
      _transientFailureCount += 1;
    }
    final startedAt = _transientFailureStartedAt ?? now;
    return now.difference(startedAt) < transientFailureTolerance;
  }

  void _clearTransientFailure() {
    _transientFailureStartedAt = null;
    _transientFailureCode = null;
    _transientFailureCount = 0;
  }

  void _debugFailure({
    required String phase,
    required String errorCode,
    required Object error,
    required Duration elapsed,
    required bool isTransient,
  }) {
    if (!enableDebugLogging) {
      return;
    }
    final detail = switch (error) {
      rust_domain.ScanError(:final message) => message.replaceAll(
        RegExp(r"[\r\n]+"),
        " ",
      ),
      _ => error.toString(),
    };
    debugPrint(
      "[Ame sync] phase=$phase result=${isTransient ? "retrying" : "failed"} "
      "elapsed_ms=${elapsed.inMilliseconds} code=$errorCode "
      "attempt=$_transientFailureCount message=$detail",
    );
  }

  void _debugSnapshot(
    String phase,
    LibrarySynchronizationSnapshot snapshot,
    Duration elapsed,
  ) {
    if (!enableDebugLogging) {
      return;
    }
    final now = _now();
    final signatures = {
      for (final entry in snapshot.roots.entries)
        entry.key:
            "${entry.value.freshness.name}:${entry.value.sourceStatus.name}:"
            "${entry.value.lastIssueCode ?? "-"}",
    };
    final hasTransition =
        signatures.length != _lastDebugRootSignatures.length ||
        signatures.entries.any(
          (entry) => _lastDebugRootSignatures[entry.key] != entry.value,
        );
    final shouldReportHeartbeat =
        _lastDebugSnapshotAt == null ||
        now.difference(_lastDebugSnapshotAt!) >= const Duration(seconds: 5);
    if (!hasTransition &&
        !shouldReportHeartbeat &&
        elapsed.inMilliseconds < 500) {
      return;
    }
    _lastDebugRootSignatures = signatures;
    _lastDebugSnapshotAt = now;
    if (snapshot.roots.isEmpty) {
      debugPrint(
        "[Ame sync] phase=$phase elapsed_ms=${elapsed.inMilliseconds} "
        "running=${snapshot.isRunning} roots=0 code=${snapshot.lastErrorCode ?? "-"}",
      );
      return;
    }
    for (final status in snapshot.roots.values) {
      final root = status.rootId.length <= 8
          ? status.rootId
          : status.rootId.substring(0, 8);
      debugPrint(
        "[Ame sync] phase=$phase elapsed_ms=${elapsed.inMilliseconds} "
        "root=$root freshness=${status.freshness.name} "
        "source=${status.sourceStatus.name} pending=${status.pendingChangeCount} "
        "retry=${status.retryWaitCount} gaps=${status.freshnessUnknownCount} "
        "code=${status.lastIssueCode ?? snapshot.lastErrorCode ?? "-"}",
      );
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
