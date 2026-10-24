import "../domain/library_organization_archive.dart";
import "library_organization_archive.dart";

sealed class LibraryArchiveRequest {
  LibraryArchiveRequest(this.path)
    : operationId =
          "archive-${DateTime.now().microsecondsSinceEpoch}-${++_sequence}";
  static int _sequence = 0;
  final String operationId;
  final String path;

  Stream<LibraryArchiveEvent> start(LibraryArchiveTask task);
  bool accepts(LibraryArchiveEvent event);
}

final class LibraryArchiveExport extends LibraryArchiveRequest {
  LibraryArchiveExport(super.path);

  @override
  Stream<LibraryArchiveEvent> start(LibraryArchiveTask task) =>
      task.export(path);
  @override
  bool accepts(LibraryArchiveEvent event) => event is LibraryArchiveExported;
}

final class LibraryArchiveInspect extends LibraryArchiveRequest {
  LibraryArchiveInspect(super.path);

  @override
  Stream<LibraryArchiveEvent> start(LibraryArchiveTask task) =>
      task.inspect(path);
  @override
  bool accepts(LibraryArchiveEvent event) => event is LibraryArchiveInspected;
}

final class LibraryArchiveRestore extends LibraryArchiveRequest {
  LibraryArchiveRestore(super.path, this.inspection);
  final LibraryArchiveInspection inspection;

  @override
  Stream<LibraryArchiveEvent> start(LibraryArchiveTask task) =>
      task.restore(operationId, path, inspection);
  @override
  bool accepts(LibraryArchiveEvent event) =>
      event is LibraryArchiveRestored && event.operationId == operationId;
}
