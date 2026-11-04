import "package:cedarflake_ame/features/library/application/ports/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";

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
