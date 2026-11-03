import "../../domain/library_organization_archive.dart";

abstract interface class LibraryOrganizationArchives {
  LibraryArchiveTask createTask();
}

abstract interface class LibraryArchiveTask {
  Stream<LibraryArchiveEvent> export(String destination);
  Stream<LibraryArchiveEvent> inspect(String path);
  Stream<LibraryArchiveEvent> restore(
    String operationId,
    String path,
    LibraryArchiveInspection inspection,
  );
  void cancel();
  void dispose();
}
