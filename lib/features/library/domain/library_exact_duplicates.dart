enum LibraryExactAnalysisPhase { running, completed, cancelled, failed }

class LibraryExactAnalysisProgress {
  const LibraryExactAnalysisProgress({
    required this.runId,
    required this.phase,
    required this.checkedFiles,
    required this.fingerprintedFiles,
    required this.skippedFiles,
    this.currentPath,
  });

  final String runId;
  final LibraryExactAnalysisPhase phase;
  final int checkedFiles;
  final int fingerprintedFiles;
  final int skippedFiles;
  final String? currentPath;
}

class LibraryDuplicateGroupCursor {
  const LibraryDuplicateGroupCursor(this.digest, this.byteLength);
  final String digest;
  final int byteLength;
}

class LibraryDuplicateGroup {
  const LibraryDuplicateGroup({
    required this.digest,
    required this.byteLength,
    required this.locationCount,
    required this.physicalFileCount,
    required this.representativeLocationId,
    required this.representativePath,
  });
  final String digest;
  final int byteLength;
  final int locationCount;
  final int physicalFileCount;
  final String representativeLocationId;
  final String representativePath;
}

class LibraryDuplicateGroupPage {
  const LibraryDuplicateGroupPage({required this.groups, this.next});
  final List<LibraryDuplicateGroup> groups;
  final LibraryDuplicateGroupCursor? next;
}

enum LibraryDuplicateAvailability { local, cloudOnly, offline, unavailable }

class LibraryDuplicateLocation {
  const LibraryDuplicateLocation({
    required this.locationId,
    required this.displayPath,
    required this.availability,
  });
  final String locationId;
  final String displayPath;
  final LibraryDuplicateAvailability availability;
}

class LibraryDuplicateLocationPage {
  const LibraryDuplicateLocationPage({
    required this.locations,
    this.nextLocationId,
  });
  final List<LibraryDuplicateLocation> locations;
  final String? nextLocationId;
}
