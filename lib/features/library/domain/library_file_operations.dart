import "library_organization.dart";
import "library_organization_batch.dart";

enum LibraryFileAction { copy, move, rename, recycle, deletePermanently }

enum LibraryFileOrganizationRule { direct, category, album, date, source }

sealed class LibraryFileSelection {
  const LibraryFileSelection();
}

class LibraryFilesInGallery extends LibraryFileSelection {
  const LibraryFilesInGallery(this.selection);
  final LibraryOrganizationSelection selection;
}

class LibraryFileGroupsInGallery extends LibraryFileSelection {
  const LibraryFileGroupsInGallery(this.selection);
  final LibraryOrganizationSelection selection;
}

class LibraryFilesAtLocations extends LibraryFileSelection {
  LibraryFilesAtLocations(Iterable<LibraryOrganizationTarget> locations)
    : locations = List.unmodifiable(locations);
  final List<LibraryOrganizationTarget> locations;
}

class LibraryFileCommand {
  const LibraryFileCommand(
    this.action, {
    this.directory,
    this.name,
    this.rule = LibraryFileOrganizationRule.direct,
  });
  final LibraryFileAction action;
  final String? directory;
  final String? name;
  final LibraryFileOrganizationRule rule;
}

class LibraryFileSelectionSummary {
  const LibraryFileSelectionSummary(
    this.files,
    this.bytes, {
    this.mergedGroups,
  });
  final int files;
  final int bytes;
  final int? mergedGroups;
}

enum LibraryFileTaskPhase {
  preparing,
  running,
  refreshing,
  stopping,
  finished,
  cancelled,
  needsAttention,
}

class LibraryFileProgress {
  const LibraryFileProgress({
    required this.phase,
    this.total = 0,
    this.processed = 0,
    this.published = 0,
    this.failed = 0,
    this.cancelled = 0,
    this.skipped = 0,
    this.export,
    this.path,
    this.copiedBytes = 0,
    this.totalBytes = 0,
    this.failure,
  });
  final LibraryFileTaskPhase phase;
  final int total;
  final int processed;
  final int published;
  final int failed;
  final int cancelled;
  final int skipped;
  final LibraryFileExport? export;
  final String? path;
  final int copiedBytes;
  final int totalBytes;
  final LibraryFileFailure? failure;
  LibraryFileProgress withPhase(LibraryFileTaskPhase phase) =>
      LibraryFileProgress(
        phase: phase,
        total: total,
        processed: processed,
        published: published,
        failed: failed,
        cancelled: cancelled,
        skipped: skipped,
        export: export,
        path: path,
        copiedBytes: copiedBytes,
        totalBytes: totalBytes,
        failure: failure,
      );
  bool get isFinished => switch (phase) {
    LibraryFileTaskPhase.finished ||
    LibraryFileTaskPhase.cancelled ||
    LibraryFileTaskPhase.needsAttention => true,
    _ => false,
  };
}

enum LibraryFileRecordPhase {
  prepared,
  executing,
  nameOccupied,
  copied,
  removingSource,
  nativeCompleted,
  published,
  failed,
  cancelled,
  uncertain,
}

class LibraryFileRecord {
  const LibraryFileRecord({
    required this.id,
    required this.action,
    required this.source,
    required this.phase,
    required this.updatedAt,
    required this.canContinueMove,
    required this.canOpenRecycleBin,
    required this.target,
    required this.canRetry,
    this.destination,
    this.detail,
    this.failure,
  });
  final BigInt id;
  final LibraryFileAction action;
  final String source;
  final String? destination;
  final LibraryFileRecordPhase phase;
  final DateTime updatedAt;
  final bool canContinueMove;
  final bool canOpenRecycleBin;
  final LibraryOrganizationTarget target;
  final bool canRetry;
  final String? detail;
  final LibraryFileFailure? failure;
}

class LibraryFileFailure implements Exception {
  const LibraryFileFailure(this.code, this.detail);
  final String code;
  final String detail;
  @override
  String toString() => "$code: $detail";
}

enum LibraryFileReportState { pending, notNeeded, written, failed }

class LibraryFileExport {
  const LibraryFileExport({
    required this.id,
    required this.batchId,
    required this.directory,
    required this.selected,
    required this.skipped,
    required this.createdAt,
    required this.reportState,
    this.reportPath,
    this.failure,
  });
  final BigInt id;
  final String batchId;
  final String directory;
  final int selected;
  final int skipped;
  final DateTime createdAt;
  final LibraryFileReportState reportState;
  final String? reportPath;
  final LibraryFileFailure? failure;
}
