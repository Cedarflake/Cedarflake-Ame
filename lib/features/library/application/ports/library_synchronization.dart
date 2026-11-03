import "../../domain/library_synchronization_models.dart";

enum LibrarySynchronizationStartResult { started, failed, skipped }

abstract interface class LibrarySynchronization {
  LibrarySynchronizationSnapshot get current;

  Stream<LibrarySynchronizationSnapshot> watch();

  Future<LibrarySynchronizationStartResult> start();

  Future<void> stop();

  Future<void> dispose();
}
