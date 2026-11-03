import "package:flutter/foundation.dart";

import "../../../../src/rust/api/synchronization.dart" as rust_api;
import "../../../../src/rust/domain.dart" as rust_domain;
import "../../../../src/rust/domain/library_synchronization.dart" as rust_sync;
import "../../application/ports/library_synchronization_gateway.dart";
import "../../domain/library_synchronization_models.dart";
import "library_synchronization_mapping.dart";

typedef NativeSynchronizationSnapshotCall =
    Future<rust_sync.LibrarySynchronizationSnapshot> Function(
      BigInt ownerTicket,
    );
typedef NativeSynchronizationStop =
    Future<void> Function(BigInt cancellationFence);

class RustLibrarySynchronizationGateway
    implements LibrarySynchronizationGateway {
  RustLibrarySynchronizationGateway.production()
    : this._(
        startCall: (ticket) =>
            rust_api.startLibrarySynchronization(ownerTicket: ticket),
        pollCall: (ticket) =>
            rust_api.pollLibrarySynchronization(ownerTicket: ticket),
        stopCall: (fence) =>
            rust_api.stopLibrarySynchronization(cancellationFence: fence),
        reserveStartTicket: rust_api.reserveLibrarySynchronizationStartTicket,
        reserveStopFence: rust_api.reserveLibrarySynchronizationStopFence,
        now: DateTime.now,
      );

  @visibleForTesting
  RustLibrarySynchronizationGateway.testing({
    required NativeSynchronizationSnapshotCall startCall,
    required NativeSynchronizationSnapshotCall pollCall,
    required NativeSynchronizationStop stopCall,
    required BigInt Function() reserveStartTicket,
    required BigInt Function() reserveStopFence,
    DateTime Function()? now,
  }) : this._(
         startCall: startCall,
         pollCall: pollCall,
         stopCall: stopCall,
         reserveStartTicket: reserveStartTicket,
         reserveStopFence: reserveStopFence,
         now: now ?? DateTime.now,
       );

  RustLibrarySynchronizationGateway._({
    required this._startCall,
    required this._pollCall,
    required this._stopCall,
    required this._reserveStartTicket,
    required this._reserveStopFence,
    required this._now,
  });

  final NativeSynchronizationSnapshotCall _startCall;
  final NativeSynchronizationSnapshotCall _pollCall;
  final NativeSynchronizationStop _stopCall;
  final BigInt Function() _reserveStartTicket;
  final BigInt Function() _reserveStopFence;
  final DateTime Function() _now;

  @override
  BigInt reserveStartTicket() => _admit(_reserveStartTicket);

  @override
  BigInt reserveStopFence() => _admit(_reserveStopFence);

  @override
  Future<LibrarySynchronizationSnapshot> start(BigInt ownerTicket) =>
      _load(_startCall, ownerTicket);

  @override
  Future<LibrarySynchronizationSnapshot> poll(BigInt ownerTicket) =>
      _load(_pollCall, ownerTicket);

  @override
  Future<void> stop(BigInt cancellationFence) =>
      _call(() => _stopCall(cancellationFence));

  BigInt _admit(BigInt Function() reserve) {
    try {
      return reserve();
    } on Object catch (error, stack) {
      _throwFailure(error, stack);
    }
  }

  Future<LibrarySynchronizationSnapshot> _load(
    NativeSynchronizationSnapshotCall call,
    BigInt ticket,
  ) => _call(() async {
    final snapshot = await call(ticket);
    return mapRustSynchronizationSnapshot(snapshot, observedAt: _now());
  });

  Future<T> _call<T>(Future<T> Function() call) async {
    try {
      return await call();
    } on Object catch (error, stack) {
      _throwFailure(error, stack);
    }
  }

  Never _throwFailure(Object error, StackTrace stack) {
    final failure = switch (error) {
      rust_domain.ScanError(:final code, :final message) =>
        LibrarySynchronizationFailure(code: code, message: message),
      _ => LibrarySynchronizationFailure(
        code: "library_synchronization_poll_failed",
        message: error.toString(),
      ),
    };
    Error.throwWithStackTrace(failure, stack);
  }
}
