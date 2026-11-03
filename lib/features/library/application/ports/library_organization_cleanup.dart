import "../../domain/library_organization_cleanup.dart";

abstract interface class LibraryOrganizationCleanup {
  LibraryCleanupTask createTask();
}

abstract interface class LibraryCleanupTask {
  Stream<LibraryCleanupEvent> inspect(int? cutoffUnixMs);
  Stream<LibraryCleanupEvent> apply(
    String operationId,
    LibraryCleanupInspection inspection,
  );
  void cancel();
  void dispose();
}
