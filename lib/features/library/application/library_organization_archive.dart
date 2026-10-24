import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/organization_archive.dart" as api;
import "../../../src/rust/domain.dart" as rust;
import "../../../src/rust/domain/organization_archive.dart" as dto;
import "../domain/library_organization_archive.dart";

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

final libraryOrganizationArchivesProvider =
    Provider<LibraryOrganizationArchives>(
      (ref) => const RustLibraryOrganizationArchives(),
    );

class RustLibraryOrganizationArchives implements LibraryOrganizationArchives {
  const RustLibraryOrganizationArchives();

  @override
  LibraryArchiveTask createTask() =>
      _RustArchiveTask(api.createOrganizationArchiveTask());
}

class _RustArchiveTask implements LibraryArchiveTask {
  _RustArchiveTask(this.task);
  final api.OrganizationArchiveTask task;

  @override
  Stream<LibraryArchiveEvent> export(String destination) => _events(
    api.exportOrganizationArchive(destination: destination, task: task),
  );

  @override
  Stream<LibraryArchiveEvent> inspect(String path) =>
      _events(api.inspectOrganizationArchive(archive: path, task: task));

  @override
  Stream<LibraryArchiveEvent> restore(
    String operationId,
    String path,
    LibraryArchiveInspection inspection,
  ) => _events(
    api.restoreOrganizationArchive(
      archive: path,
      task: task,
      request: dto.OrganizationArchiveRestoreRequest(
        operationId: operationId,
        archiveId: inspection.archiveId,
        checksum: inspection.checksum,
        expectedCatalogRevision: inspection.catalogRevision,
        expectedOrganizationRevision: inspection.organizationRevision,
      ),
    ),
  );

  @override
  void cancel() => task.cancel();

  @override
  void dispose() => task.dispose();
}

Stream<LibraryArchiveEvent> _events(
  Stream<dto.OrganizationArchiveEvent> source,
) async* {
  try {
    await for (final event in source) {
      yield switch (event) {
        dto.OrganizationArchiveEvent_Progress(:final progress) =>
          LibraryArchiveProgress(switch (progress.phase) {
            dto.OrganizationArchivePhase.snapshot =>
              LibraryArchivePhase.snapshot,
            dto.OrganizationArchivePhase.writing => LibraryArchivePhase.writing,
            dto.OrganizationArchivePhase.verifying =>
              LibraryArchivePhase.verifying,
            dto.OrganizationArchivePhase.matching =>
              LibraryArchivePhase.matching,
            dto.OrganizationArchivePhase.restoring =>
              LibraryArchivePhase.restoring,
          }),
        dto.OrganizationArchiveEvent_Completed() =>
          const LibraryArchiveExported(),
        dto.OrganizationArchiveEvent_Inspected(:final inspection) =>
          LibraryArchiveInspected(
            LibraryArchiveInspection(
              archiveId: inspection.archiveId,
              checksum: inspection.checksum,
              catalogRevision: inspection.associations.catalogRevision,
              organizationRevision:
                  inspection.associations.organizationRevision,
              createdUnixMs: inspection.createdUnixMs,
              albums: inspection.activeAlbums.toInt(),
              groups: inspection.groups.toInt(),
              favorites: inspection.favoriteGroups.toInt(),
              conflicts: inspection.conflictingGroups.toInt(),
              duplicateChoices: inspection.duplicateChoices.toInt(),
              matchedGroups: inspection.associations.matchedGroups.toInt(),
              unmatchedGroups: inspection.associations.unmatchedGroups.toInt(),
              unresolvedPreferredCopies: inspection
                  .associations
                  .unresolvedPreferredCopies
                  .toInt(),
              hasContinuation: inspection.hasContinuation,
            ),
          ),
        dto.OrganizationArchiveEvent_Restored(:final receipt) =>
          LibraryArchiveRestored(receipt.operationId),
        dto.OrganizationArchiveEvent_Cancelled() =>
          const LibraryArchiveCancelled(),
        dto.OrganizationArchiveEvent_Failed(:final code, :final message) =>
          throw LibraryArchiveFailure(code, message),
      };
    }
  } on rust.ScanError catch (error) {
    throw LibraryArchiveFailure(error.code, error.message);
  }
}
