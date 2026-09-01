import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/synchronization.dart" as rust_api;
import "../../../src/rust/domain.dart" as rust_domain;
import "../../../src/rust/domain/library_change.dart" as rust_change;
import "../../../src/rust/domain/library_synchronization.dart" as rust_sync;
import "../../../src/rust/domain/persistent_journal.dart" as rust_journal;
import "../domain/library_models.dart";
import "../domain/library_synchronization_models.dart";
import "library_synchronization_policy.g.dart";

enum LibrarySynchronizationStartResult { started, failed, skipped }

abstract interface class LibrarySynchronization {
  LibrarySynchronizationSnapshot get current;

  Stream<LibrarySynchronizationSnapshot> watch();

  Future<LibrarySynchronizationStartResult> start();

  Future<void> stop();

  Future<void> dispose();
}

class InertLibrarySynchronization implements LibrarySynchronization {
  InertLibrarySynchronization();

  final LibrarySynchronizationSnapshot _snapshot =
      LibrarySynchronizationSnapshot.stopped();

  @override
  LibrarySynchronizationSnapshot get current => _snapshot;

  @override
  Future<LibrarySynchronizationStartResult> start() async =>
      LibrarySynchronizationStartResult.skipped;

  @override
  Future<void> stop() async {}

  @override
  Future<void> dispose() async {}

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => const Stream.empty();
}

typedef RustSynchronizationCall =
    Future<rust_sync.LibrarySynchronizationSnapshot> Function();
typedef RustSynchronizationStop = Future<void> Function();
typedef RustSynchronizationTicketCall = BigInt Function();
typedef _RustSynchronizationOwnedCall =
    Future<rust_sync.LibrarySynchronizationSnapshot> Function(BigInt);
typedef _RustSynchronizationFencedStop = Future<void> Function(BigInt);
typedef SynchronizationClock = DateTime Function();

class RustLibrarySynchronization implements LibrarySynchronization {
  RustLibrarySynchronization.production() : this._();

  @visibleForTesting
  RustLibrarySynchronization.testing({
    RustSynchronizationCall? startCall,
    RustSynchronizationCall? pollCall,
    RustSynchronizationStop? stopCall,
    RustSynchronizationTicketCall? reserveStartTicketCall,
    RustSynchronizationTicketCall? reserveStopFenceCall,
    Duration pollInterval = productionLibrarySynchronizationPollInterval,
    Duration transientFailureTolerance = const Duration(seconds: 30),
    List<Duration> startRetryDelays = const [
      Duration(milliseconds: 100),
      Duration(milliseconds: 500),
      Duration(seconds: 2),
    ],
    SynchronizationClock? now,
    bool enableDebugLogging = kDebugMode,
  }) : this._(
         startCall: startCall,
         pollCall: pollCall,
         stopCall: stopCall,
         reserveStartTicketCall: reserveStartTicketCall,
         reserveStopFenceCall: reserveStopFenceCall,
         pollInterval: pollInterval,
         transientFailureTolerance: transientFailureTolerance,
         startRetryDelays: startRetryDelays,
         now: now,
         enableDebugLogging: enableDebugLogging,
       );

  RustLibrarySynchronization._({
    RustSynchronizationCall? startCall,
    RustSynchronizationCall? pollCall,
    RustSynchronizationStop? stopCall,
    RustSynchronizationTicketCall? reserveStartTicketCall,
    RustSynchronizationTicketCall? reserveStopFenceCall,
    this._pollInterval = productionLibrarySynchronizationPollInterval,
    this._transientFailureTolerance = const Duration(seconds: 30),
    List<Duration> startRetryDelays = const [
      Duration(milliseconds: 100),
      Duration(milliseconds: 500),
      Duration(seconds: 2),
    ],
    SynchronizationClock? now,
    this._enableDebugLogging = kDebugMode,
  }) : assert(startRetryDelays.every((delay) => !delay.isNegative)),
       _startRetryDelays = List.unmodifiable(startRetryDelays),
       _now = now ?? DateTime.now {
    final localTickets =
        startCall != null || pollCall != null || stopCall != null
        ? _LocalLifecycleTicketAllocator()
        : null;
    _reserveStartTicket =
        reserveStartTicketCall ??
        localTickets?.reserve ??
        rust_api.reserveLibrarySynchronizationStartTicket;
    _reserveStopFence =
        reserveStopFenceCall ??
        localTickets?.reserve ??
        rust_api.reserveLibrarySynchronizationStopFence;
    _startCall = startCall == null
        ? (ownerTicket) =>
              rust_api.startLibrarySynchronization(ownerTicket: ownerTicket)
        : (_) => startCall();
    _pollCall = pollCall == null
        ? (ownerTicket) =>
              rust_api.pollLibrarySynchronization(ownerTicket: ownerTicket)
        : (_) => pollCall();
    _stopCall = stopCall == null
        ? (cancellationFence) => rust_api.stopLibrarySynchronization(
            cancellationFence: cancellationFence,
          )
        : (_) => stopCall();
  }

  late final RustSynchronizationTicketCall _reserveStartTicket;
  late final RustSynchronizationTicketCall _reserveStopFence;
  late final _RustSynchronizationOwnedCall _startCall;
  late final _RustSynchronizationOwnedCall _pollCall;
  late final _RustSynchronizationFencedStop _stopCall;
  final Duration _pollInterval;
  final Duration _transientFailureTolerance;
  final List<Duration> _startRetryDelays;
  final SynchronizationClock _now;
  final bool _enableDebugLogging;
  final StreamController<LibrarySynchronizationSnapshot> _updates =
      StreamController.broadcast(sync: true);
  LibrarySynchronizationSnapshot _current =
      LibrarySynchronizationSnapshot.stopped();
  Timer? _timer;
  Timer? _startRetryTimer;
  Completer<void>? _startRetryWake;
  Future<LibrarySynchronizationStartResult>? _startOperation;
  int? _startOperationGeneration;
  Future<void>? _activePoll;
  bool _isStarted = false;
  BigInt? _ownerTicket;
  bool _isStopping = false;
  bool _isDisposed = false;
  int _generation = 0;
  Future<void>? _stopOperation;
  Future<void>? _disposeOperation;
  DateTime? _transientFailureStartedAt;
  String? _transientFailureCode;
  int _transientFailureCount = 0;
  DateTime? _lastDebugSnapshotAt;
  Map<String, String> _lastDebugRootSignatures = const {};

  @override
  LibrarySynchronizationSnapshot get current => _current;

  @override
  Future<LibrarySynchronizationStartResult> start() {
    if (_isDisposed || _isStopping) {
      return Future.value(LibrarySynchronizationStartResult.skipped);
    }
    if (_isStarted) {
      return Future.value(LibrarySynchronizationStartResult.started);
    }
    final activeOperation = _startOperation;
    if (activeOperation != null && _startOperationGeneration == _generation) {
      return activeOperation;
    }
    _startOperation = null;
    _startOperationGeneration = null;

    late final BigInt ownerTicket;
    try {
      ownerTicket = _reserveStartTicket();
    } on Object catch (error) {
      final errorCode = _errorCode(error);
      _debugFailure(
        phase: "start-admission",
        errorCode: errorCode,
        error: error,
        elapsed: Duration.zero,
        isTransient: false,
      );
      _publish(
        _current.degraded(errorCode, occurredAt: _now()),
        phase: "start-admission",
        elapsed: Duration.zero,
      );
      return Future.value(LibrarySynchronizationStartResult.failed);
    }
    final generation = ++_generation;
    _ownerTicket = ownerTicket;
    final operation = _start(generation, ownerTicket);
    late final Future<LibrarySynchronizationStartResult> trackedOperation;
    trackedOperation = operation.whenComplete(() {
      if (identical(_startOperation, trackedOperation)) {
        _startOperation = null;
        _startOperationGeneration = null;
      }
    });
    _startOperation = trackedOperation;
    _startOperationGeneration = generation;
    return trackedOperation;
  }

  Future<LibrarySynchronizationStartResult> _start(
    int generation,
    BigInt ownerTicket,
  ) async {
    for (var attempt = 0; ; attempt += 1) {
      if (!_isCurrentStartGeneration(generation)) {
        return LibrarySynchronizationStartResult.skipped;
      }
      final result = await _runCall(
        _startCall,
        ownerTicket,
        phase: "start",
        generation: generation,
      );
      if (result.status == _SynchronizationCallStatus.stale ||
          !_isCurrentStartGeneration(generation)) {
        return LibrarySynchronizationStartResult.skipped;
      }
      if (result.status == _SynchronizationCallStatus.succeeded) {
        _isStarted = true;
        _timer?.cancel();
        _timer = Timer.periodic(_pollInterval, (_) => unawaited(_pollOnce()));
        return LibrarySynchronizationStartResult.started;
      }
      _isStarted = false;
      if (!result.isTransient) {
        return LibrarySynchronizationStartResult.failed;
      }
      if (attempt >= _startRetryDelays.length) {
        final errorCode = result.errorCode;
        if (result.isTransient && errorCode != null) {
          _clearTransientFailure();
          _publish(
            _current.degraded(errorCode, occurredAt: _now()),
            phase: "start",
            elapsed: Duration.zero,
          );
        }
        return LibrarySynchronizationStartResult.failed;
      }
      final shouldRetry = await _waitForStartRetry(
        _startRetryDelays[attempt],
        generation,
      );
      if (!shouldRetry) {
        return LibrarySynchronizationStartResult.skipped;
      }
    }
  }

  Future<void> _pollOnce() {
    if (!_isStarted || _isStopping) {
      return Future.value();
    }
    final activePoll = _activePoll;
    if (activePoll != null) {
      return activePoll;
    }
    final generation = _generation;
    final ownerTicket = _ownerTicket;
    if (ownerTicket == null) {
      return Future.value();
    }
    final operation = _runCall(
      _pollCall,
      ownerTicket,
      phase: "poll",
      generation: generation,
    ).then<void>((_) {});
    _activePoll = operation;
    return operation.whenComplete(() {
      if (identical(_activePoll, operation)) {
        _activePoll = null;
      }
    });
  }

  Future<_SynchronizationCallResult> _runCall(
    _RustSynchronizationOwnedCall call,
    BigInt ownerTicket, {
    required String phase,
    required int generation,
  }) async {
    final stopwatch = Stopwatch()..start();
    try {
      final snapshot = _retainUnresolvedFailures(
        _mapSnapshot(await call(ownerTicket)),
      );
      if (generation != _generation) {
        return const _SynchronizationCallResult.stale();
      }
      _clearTransientFailure();
      _publish(snapshot, phase: phase, elapsed: stopwatch.elapsed);
      return const _SynchronizationCallResult.succeeded();
    } on Object catch (error) {
      if (generation != _generation) {
        return const _SynchronizationCallResult.stale();
      }
      final errorCode = _errorCode(error);
      if (_shouldRetryTransiently(errorCode)) {
        _debugFailure(
          phase: phase,
          errorCode: errorCode,
          error: error,
          elapsed: stopwatch.elapsed,
          isTransient: true,
        );
        return _SynchronizationCallResult.failed(
          errorCode: errorCode,
          isTransient: true,
        );
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
        _current.degraded(errorCode, occurredAt: _now()),
        phase: phase,
        elapsed: stopwatch.elapsed,
      );
      return _SynchronizationCallResult.failed(
        errorCode: errorCode,
        isTransient: false,
      );
    }
  }

  bool _isCurrentStartGeneration(int generation) =>
      !_isDisposed && !_isStopping && generation == _generation;

  Future<bool> _waitForStartRetry(Duration delay, int generation) async {
    if (!_isCurrentStartGeneration(generation)) {
      return false;
    }
    final wake = Completer<void>();
    final timer = Timer(delay, () {
      if (!wake.isCompleted) {
        wake.complete();
      }
    });
    _startRetryTimer = timer;
    _startRetryWake = wake;
    await wake.future;
    if (identical(_startRetryTimer, timer)) {
      _startRetryTimer = null;
    }
    if (identical(_startRetryWake, wake)) {
      _startRetryWake = null;
    }
    return _isCurrentStartGeneration(generation);
  }

  void _cancelStartRetry() {
    _startRetryTimer?.cancel();
    _startRetryTimer = null;
    final wake = _startRetryWake;
    _startRetryWake = null;
    if (wake != null && !wake.isCompleted) {
      wake.complete();
    }
  }

  @override
  Future<void> stop() {
    final activeOperation = _stopOperation;
    if (activeOperation != null) {
      return activeOperation;
    }

    _isStopping = true;
    _generation += 1;
    _cancelStartRetry();
    _timer?.cancel();
    _timer = null;
    _activePoll = null;
    _startOperation = null;
    _startOperationGeneration = null;

    late final Future<void> operation;
    try {
      final cancellationFence = _reserveStopFence();
      operation = _stop(cancellationFence);
    } on Object catch (error, stackTrace) {
      operation = Future.error(error, stackTrace);
    }
    late final Future<void> trackedOperation;
    trackedOperation = operation.whenComplete(() {
      if (identical(_stopOperation, trackedOperation)) {
        _stopOperation = null;
        _isStopping = false;
      }
    });
    _stopOperation = trackedOperation;
    return trackedOperation;
  }

  Future<void> _stop(BigInt cancellationFence) async {
    try {
      await _stopCall(cancellationFence);
    } on rust_domain.ScanError catch (error) {
      if (error.code != "library_synchronization_not_started") {
        rethrow;
      }
    }
    _ownerTicket = null;
    _isStarted = false;
    _clearTransientFailure();
    _publish(_current.stopped(), phase: "stop", elapsed: Duration.zero);
  }

  @override
  Future<void> dispose() {
    final activeOperation = _disposeOperation;
    if (activeOperation != null) {
      return activeOperation;
    }
    _isDisposed = true;
    final operation = _dispose();
    late final Future<void> trackedOperation;
    trackedOperation = operation.whenComplete(() {
      if (identical(_disposeOperation, trackedOperation)) {
        _disposeOperation = null;
      }
    });
    _disposeOperation = trackedOperation;
    return trackedOperation;
  }

  Future<void> _dispose() async {
    await stop();
    await _updates.close();
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
          previous?.rootGeneration == current.rootGeneration &&
          current.freshness == LibraryCatalogFreshness.updating) {
        retainedFailure = true;
        roots[entry.key] = LibraryRootSynchronizationStatus(
          rootId: current.rootId,
          rootGeneration: current.rootGeneration,
          availability: current.availability,
          freshness: LibraryCatalogFreshness.needsReconciliation,
          freshnessCause: previous!.freshnessCause,
          continuity: current.continuity,
          phase: previous.phase,
          phaseStartedAt: previous.phaseStartedAt,
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
    return now.difference(startedAt) < _transientFailureTolerance;
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
    if (!_enableDebugLogging) {
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
    if (!_enableDebugLogging) {
      return;
    }
    final now = _now();
    final signatures = {
      for (final entry in snapshot.roots.entries)
        entry.key:
            "${entry.value.freshness.name}:${entry.value.continuity.name}:"
            "${entry.value.sourceStatus.name}:"
            "${entry.value.phase.name}:${entry.value.lastIssueCode ?? "-"}",
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
      final phaseElapsed = now.difference(status.phaseStartedAt);
      debugPrint(
        "[Ame sync] phase=$phase elapsed_ms=${elapsed.inMilliseconds} "
        "root=$root freshness=${status.freshness.name} "
        "root_phase=${status.phase.name} "
        "root_phase_elapsed_ms=${phaseElapsed.isNegative ? 0 : phaseElapsed.inMilliseconds} "
        "source=${status.sourceStatus.name} pending=${status.pendingChangeCount} "
        "retry=${status.retryWaitCount} gaps=${status.freshnessUnknownCount} "
        "code=${status.lastIssueCode ?? snapshot.lastErrorCode ?? "-"}",
      );
    }
  }

  LibrarySynchronizationSnapshot _mapSnapshot(
    rust_sync.LibrarySynchronizationSnapshot snapshot,
  ) {
    final observedAt = _now();
    return LibrarySynchronizationSnapshot(
      isRunning: snapshot.isRunning,
      catalogRevision: snapshot.catalogRevision,
      appliedMutationCount: snapshot.appliedMutationCount,
      roots: {
        for (final root in snapshot.roots)
          root.rootId: _mapRootStatus(root, observedAt),
      },
    );
  }

  LibraryRootSynchronizationStatus _mapRootStatus(
    rust_sync.LibraryRootSynchronizationStatus root,
    DateTime observedAt,
  ) {
    final phase = switch (root.phase) {
      rust_sync.LibrarySynchronizationPhase.watcherStartup =>
        LibrarySynchronizationPhase.watcherStartup,
      rust_sync.LibrarySynchronizationPhase.inventoryEnumeration =>
        LibrarySynchronizationPhase.inventoryEnumeration,
      rust_sync.LibrarySynchronizationPhase.inventoryComparison =>
        LibrarySynchronizationPhase.inventoryComparison,
      rust_sync.LibrarySynchronizationPhase.queuePublication =>
        LibrarySynchronizationPhase.queuePublication,
      rust_sync.LibrarySynchronizationPhase.retryWait =>
        LibrarySynchronizationPhase.retryWait,
      rust_sync.LibrarySynchronizationPhase.reconciliation =>
        LibrarySynchronizationPhase.reconciliation,
      rust_sync.LibrarySynchronizationPhase.fullScan =>
        LibrarySynchronizationPhase.fullScan,
      rust_sync.LibrarySynchronizationPhase.blocked =>
        LibrarySynchronizationPhase.blocked,
      rust_sync.LibrarySynchronizationPhase.synchronized =>
        LibrarySynchronizationPhase.synchronized,
      rust_sync.LibrarySynchronizationPhase.unavailable =>
        LibrarySynchronizationPhase.unavailable,
    };
    final previous = _current.roots[root.rootId];
    final phaseStartedAt =
        previous != null &&
            previous.rootGeneration == root.rootGeneration &&
            previous.phase == phase
        ? previous.phaseStartedAt
        : observedAt;
    return LibraryRootSynchronizationStatus(
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
      continuity: switch (root.continuity) {
        rust_journal.PersistentJournalContinuityState.baselineRequired =>
          LibraryContinuityState.baselineRequired,
        rust_journal.PersistentJournalContinuityState.catchingUp =>
          LibraryContinuityState.catchingUp,
        rust_journal.PersistentJournalContinuityState.current =>
          LibraryContinuityState.current,
        rust_journal.PersistentJournalContinuityState.recoveryRequired =>
          LibraryContinuityState.recoveryRequired,
        rust_journal.PersistentJournalContinuityState.liveOnly =>
          LibraryContinuityState.liveOnly,
        rust_journal.PersistentJournalContinuityState.unavailable =>
          LibraryContinuityState.unavailable,
      },
      phase: phase,
      phaseStartedAt: phaseStartedAt,
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
      recoveryBlocked: root.recoveryBlocked,
      lastIssueCode: root.lastIssueCode,
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

enum _SynchronizationCallStatus { succeeded, failed, stale }

class _SynchronizationCallResult {
  const _SynchronizationCallResult.succeeded()
    : status = _SynchronizationCallStatus.succeeded,
      errorCode = null,
      isTransient = false;

  const _SynchronizationCallResult.failed({
    required this.errorCode,
    required this.isTransient,
  }) : status = _SynchronizationCallStatus.failed;

  const _SynchronizationCallResult.stale()
    : status = _SynchronizationCallStatus.stale,
      errorCode = null,
      isTransient = false;

  final _SynchronizationCallStatus status;
  final String? errorCode;
  final bool isTransient;
}

class _LocalLifecycleTicketAllocator {
  BigInt _next = BigInt.zero;

  BigInt reserve() {
    _next += BigInt.one;
    return _next;
  }
}

final librarySynchronizationProvider = Provider<LibrarySynchronization>((ref) {
  return InertLibrarySynchronization();
});
