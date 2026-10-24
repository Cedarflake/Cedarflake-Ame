enum LibraryArchivePhase { snapshot, writing, verifying, matching, restoring }

class LibraryArchiveInspection {
  const LibraryArchiveInspection({
    required this.archiveId,
    required this.checksum,
    required this.catalogRevision,
    required this.organizationRevision,
    required this.createdUnixMs,
    required this.albums,
    required this.groups,
    required this.favorites,
    required this.conflicts,
    required this.duplicateChoices,
    required this.matchedGroups,
    required this.unmatchedGroups,
    required this.unresolvedPreferredCopies,
    required this.hasContinuation,
  });

  final String archiveId;
  final String checksum;
  final BigInt catalogRevision;
  final BigInt organizationRevision;
  final BigInt createdUnixMs;
  final int albums;
  final int groups;
  final int favorites;
  final int conflicts;
  final int duplicateChoices;
  final int matchedGroups;
  final int unmatchedGroups;
  final int unresolvedPreferredCopies;
  final bool hasContinuation;
}

sealed class LibraryArchiveEvent {
  const LibraryArchiveEvent();
}

final class LibraryArchiveProgress extends LibraryArchiveEvent {
  const LibraryArchiveProgress(this.phase);
  final LibraryArchivePhase phase;
}

final class LibraryArchiveExported extends LibraryArchiveEvent {
  const LibraryArchiveExported();
}

final class LibraryArchiveInspected extends LibraryArchiveEvent {
  const LibraryArchiveInspected(this.inspection);
  final LibraryArchiveInspection inspection;
}

final class LibraryArchiveRestored extends LibraryArchiveEvent {
  const LibraryArchiveRestored(this.operationId);
  final String operationId;
}

final class LibraryArchiveCancelled extends LibraryArchiveEvent {
  const LibraryArchiveCancelled();
}

class LibraryArchiveFailure implements Exception {
  const LibraryArchiveFailure(this.code, this.message);
  final String code;
  final String message;

  bool get requiresInspection => switch (code) {
    "organization_archive_inspection_changed" ||
    "organization_archive_input_changed" => true,
    _ => false,
  };

  @override
  String toString() => "$code: $message";
}
