import "../../domain/library_synchronization_models.dart";

abstract interface class LibrarySynchronizationGateway {
  BigInt reserveStartTicket();
  BigInt reserveStopFence();
  Future<LibrarySynchronizationSnapshot> start(BigInt ownerTicket);
  Future<LibrarySynchronizationSnapshot> poll(BigInt ownerTicket);
  Future<void> stop(BigInt cancellationFence);
}

class LibrarySynchronizationFailure implements Exception {
  const LibrarySynchronizationFailure({
    required this.code,
    required this.message,
  });

  final String code;
  final String message;

  @override
  String toString() => "$code: $message";
}
