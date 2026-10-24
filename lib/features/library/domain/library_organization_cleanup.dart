class LibraryCleanupInspection {
  const LibraryCleanupInspection({
    required this.cutoffUnixMs,
    required this.inspectedUnixMs,
    required this.catalogRevision,
    required this.organizationRevision,
    required this.selectionDigest,
    required this.groups,
    required this.albumGroups,
    required this.favoriteGroups,
    required this.progressGroups,
    required this.duplicateChoices,
    this.oldestUnassociatedUnixMs,
    this.newestUnassociatedUnixMs,
  });

  final int cutoffUnixMs;
  final int inspectedUnixMs;
  final BigInt catalogRevision;
  final BigInt organizationRevision;
  final String selectionDigest;
  final int groups;
  final int albumGroups;
  final int favoriteGroups;
  final int progressGroups;
  final int duplicateChoices;
  final int? oldestUnassociatedUnixMs;
  final int? newestUnassociatedUnixMs;
  int get total => groups + duplicateChoices;
}

enum LibraryCleanupPhase { inspecting, applying }

sealed class LibraryCleanupEvent {
  const LibraryCleanupEvent();
}

final class LibraryCleanupProgress extends LibraryCleanupEvent {
  const LibraryCleanupProgress(this.phase);
  final LibraryCleanupPhase phase;
}

final class LibraryCleanupInspected extends LibraryCleanupEvent {
  const LibraryCleanupInspected(this.inspection);
  final LibraryCleanupInspection inspection;
}

final class LibraryCleanupCompleted extends LibraryCleanupEvent {
  const LibraryCleanupCompleted(this.operationId, this.removedRecords);
  final String operationId;
  final int removedRecords;
}

final class LibraryCleanupCancelled extends LibraryCleanupEvent {
  const LibraryCleanupCancelled();
}

class LibraryCleanupFailure implements Exception {
  const LibraryCleanupFailure(this.code, this.message);
  final String code;
  final String message;
  bool get requiresInspection =>
      code == "organization_cleanup_changed" ||
      code == "organization_cleanup_request_invalid" ||
      code == "organization_selection_changed";

  @override
  String toString() => "$code: $message";
}
