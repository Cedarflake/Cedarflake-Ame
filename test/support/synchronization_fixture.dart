import "package:cedarflake_ame/features/library/adapters/rust/rust_library_synchronization_gateway.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization_policy.g.dart";
import "package:cedarflake_ame/src/rust/domain/library_synchronization.dart"
    as rust_sync;
import "package:flutter/foundation.dart";

LibrarySynchronizationController createTestLibrarySynchronization({
  Future<rust_sync.LibrarySynchronizationSnapshot> Function()? startCall,
  Future<rust_sync.LibrarySynchronizationSnapshot> Function()? pollCall,
  Future<void> Function()? stopCall,
  BigInt Function()? reserveStartTicketCall,
  BigInt Function()? reserveStopFenceCall,
  Duration pollInterval = productionLibrarySynchronizationPollInterval,
  Duration transientFailureTolerance = const Duration(seconds: 30),
  List<Duration> startRetryDelays = const [
    Duration(milliseconds: 100),
    Duration(milliseconds: 500),
    Duration(seconds: 2),
  ],
  DateTime Function()? now,
  bool enableDebugLogging = kDebugMode,
}) {
  var ticket = BigInt.zero;
  BigInt reserve() => ticket += BigInt.one;
  final gateway = RustLibrarySynchronizationGateway.testing(
    startCall: (_) =>
        startCall?.call() ?? (throw StateError("Missing fixture start")),
    pollCall: (_) =>
        pollCall?.call() ?? (throw StateError("Missing fixture poll")),
    stopCall: (_) =>
        stopCall?.call() ?? (throw StateError("Missing fixture stop")),
    reserveStartTicket: reserveStartTicketCall ?? reserve,
    reserveStopFence: reserveStopFenceCall ?? reserve,
    now: now,
  );
  return LibrarySynchronizationController.testing(
    gateway: gateway,
    pollInterval: pollInterval,
    transientFailureTolerance: transientFailureTolerance,
    startRetryDelays: startRetryDelays,
    now: now,
    enableDebugLogging: enableDebugLogging,
  );
}
