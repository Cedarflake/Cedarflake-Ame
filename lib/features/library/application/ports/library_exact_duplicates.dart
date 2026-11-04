import "../../domain/library_exact_duplicates.dart";

abstract interface class LibraryExactDuplicates {
  Stream<LibraryExactAnalysisProgress> analyze(String runId);
  Future<void> cancel(String runId);
  Future<LibraryDuplicateGroupPage> loadGroups(
    LibraryDuplicateGroupCursor? after,
  );
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  );
}
