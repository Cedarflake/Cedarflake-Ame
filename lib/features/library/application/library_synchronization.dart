import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_synchronization_models.dart";
import "library_synchronization_diagnostics.dart";
import "library_synchronization_failure_retention.dart";
import "library_synchronization_phase_projection.dart";
import "library_synchronization_policy.g.dart";
import "ports/library_synchronization.dart";
import "ports/library_synchronization_gateway.dart";

class LibrarySynchronizationController implements LibrarySynchronization {
  LibrarySynchronizationController.production(
    LibrarySynchronizationGateway gateway,
  ) : this._(gateway: gateway);

  @visibleForTesting
  LibrarySynchronizationController.testing({
    required LibrarySynchronizationGateway gateway,
    Duration pollInterval = productionLibrarySynchronizationPollInterval,
    Duration transientFailureTolerance = const Duration(seconds: 30),
    List<Duration> startRetryDelays = const [
      Duration(milliseconds: 100),
      Duration(milliseconds: 500),
      Duration(seconds: 2),
    ],
    DateTime Function()? now,
    bool enableDebugLogging = kDebugMode,
  }) : this._(
         gateway: gateway,
         pollInterval: pollInterval,
         transientFailureTolerance: transientFailureTolerance,
         startRetryDelays: startRetryDelays,
         now: now,
         enableDebugLogging: enableDebugLogging,
       );

  LibrarySynchronizationController._({
    required this._gateway,
    this._pollInterval = productionLibrarySynchronizationPollInterval,
    this._transientFailureTolerance = const Duration(seconds: 30),
    List<Duration> startRetryDelays = const [
      Duration(milliseconds: 100),
      Duration(milliseconds: 500),
      Duration(seconds: 2),
    ],
    DateTime Function()? now,
    bool enableDebugLogging = kDebugMode,
  }) : assert(startRetryDelays.every((delay) => !delay.isNegative)),
       _startRetryDelays = List.unmodifiable(startRetryDelays),
       _now = now ?? DateTime.now,
       _diagnostics = LibrarySynchronizationDiagnostics(
         enabled: enableDebugLogging,
         now: now ?? DateTime.now,
       );

  final LibrarySynchronizationGateway _gateway;
  final LibrarySynchronizationDiagnostics _diagnostics;
  final Duration _pollInterval;
  final Duration _transientFailureTolerance;
  final List<Duration> _startRetryDelays;
  final DateTime Function() _now;
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
      ownerTicket = _gateway.reserveStartTicket();
    } on Object catch (error) {
      final errorCode = _errorCode(error);
      _diagnostics.failure(
        attempt: _transientFailureCount,
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
        _gateway.start,
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
      _gateway.poll,
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
    Future<LibrarySynchronizationSnapshot> Function(BigInt) call,
    BigInt ownerTicket, {
    required String phase,
    required int generation,
  }) async {
    final stopwatch = Stopwatch()..start();
    try {
      final incoming = retainSynchronizationPhaseStarts(
        previous: _current,
        incoming: await call(ownerTicket),
      );
      final snapshot = retainUnresolvedSynchronizationFailures(
        previous: _current,
        incoming: incoming,
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
        _diagnostics.failure(
          attempt: _transientFailureCount,
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
      _diagnostics.failure(
        attempt: _transientFailureCount,
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
      final cancellationFence = _gateway.reserveStopFence();
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
      await _gateway.stop(cancellationFence);
    } on LibrarySynchronizationFailure catch (error) {
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
      _diagnostics.snapshot(phase, snapshot, elapsed);
      return;
    }
    _current = snapshot;
    _diagnostics.snapshot(phase, snapshot, elapsed);
    if (!_updates.isClosed) {
      _updates.add(snapshot);
    }
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

  String _errorCode(Object error) {
    if (error case LibrarySynchronizationFailure(:final code)) {
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

final librarySynchronizationProvider = Provider<LibrarySynchronization>((ref) {
  throw StateError(
    "LibrarySynchronization must be bound during application startup",
  );
});
