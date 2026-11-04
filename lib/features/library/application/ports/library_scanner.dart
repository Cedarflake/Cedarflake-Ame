import "../../domain/library_models.dart";

abstract interface class LibraryScanner {
  Future<RecoverableLibraryScan?> loadRecoverableScan();

  Future<RecoverableLibraryScan?> loadPausedScan();

  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  });

  Stream<LibraryScanUpdate> resume({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  });

  bool cancel(String scanId);

  Future<void> cancelRetainedScan(String scanId);

  bool pause(String scanId);

  bool suspend(String scanId);
}

abstract interface class LibraryRootRelocationScanner {
  Stream<LibraryScanUpdate> relocate({
    required LibraryRoot root,
    required String scanId,
    required String rootPath,
    required int previewEdge,
  });
}
