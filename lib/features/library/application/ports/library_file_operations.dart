import "../../domain/library_file_operations.dart";
import "../../domain/library_organization.dart";

abstract interface class LibraryFileOperationPlatform {
  Future<BigInt> ownerWindow();
  Future<void> openRecycleBin();
}

abstract interface class LibraryFileOperations {
  Future<LibraryFileSelectionDraft> open(LibraryFileSelection selection);
  Future<List<LibraryFileRecord>> history({BigInt? before, int limit = 40});
  Future<List<LibraryFileExport>> exports({BigInt? before, int limit = 40});
  Future<LibraryFileTask> writeReport(String batchId);
  Future<LibraryFileTask> recover(
    List<BigInt> ids, {
    required bool continueMove,
  });
  Future<void> openRecycleBin();
}

abstract interface class LibraryFileSelectionDraft {
  Future<void> appendIds(List<String> ids);
  Future<void> appendGroups(List<LibraryCleanupGroup> groups);
  Future<void> appendLocations(List<LibraryOrganizationTarget> locations);
  Future<LibraryFileSelectionSummary> prepare();
  Future<LibraryFileTask> execute(String batchId, LibraryFileCommand command);
  Future<LibraryFileProgress> progress();
  Future<void> cancel();
  Future<void> close();
}

abstract interface class LibraryFileTask {
  Future<LibraryFileProgress> progress();
  Future<void> cancel();
  Future<void> close();
}
