import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../src/rust/api/organization_cleanup.dart" as api;
import "../../../src/rust/domain.dart" as rust;
import "../../../src/rust/domain/organization_cleanup.dart" as dto;
import "../domain/library_organization_cleanup.dart";

abstract interface class LibraryOrganizationCleanup {
  LibraryCleanupTask createTask();
}

abstract interface class LibraryCleanupTask {
  Stream<LibraryCleanupEvent> inspect(int? cutoffUnixMs);
  Stream<LibraryCleanupEvent> apply(
    String operationId,
    LibraryCleanupInspection inspection,
  );
  void cancel();
  void dispose();
}

final libraryOrganizationCleanupProvider = Provider<LibraryOrganizationCleanup>(
  (ref) => const RustLibraryOrganizationCleanup(),
);

class RustLibraryOrganizationCleanup implements LibraryOrganizationCleanup {
  const RustLibraryOrganizationCleanup();
  @override
  LibraryCleanupTask createTask() =>
      _RustCleanupTask(api.createOrganizationCleanupTask());
}

class _RustCleanupTask implements LibraryCleanupTask {
  _RustCleanupTask(this.task);
  final api.OrganizationCleanupTask task;

  @override
  Stream<LibraryCleanupEvent> inspect(int? cutoffUnixMs) => _events(
    api.inspectOrganizationCleanup(cutoffUnixMs: cutoffUnixMs, task: task),
  );

  @override
  Stream<LibraryCleanupEvent> apply(
    String operationId,
    LibraryCleanupInspection inspection,
  ) => _events(
    api.applyOrganizationCleanup(
      task: task,
      request: dto.OrganizationCleanupRequest(
        operationId: operationId,
        inspection: dto.OrganizationCleanupInspection(
          cutoffUnixMs: inspection.cutoffUnixMs,
          inspectedUnixMs: inspection.inspectedUnixMs,
          catalogRevision: inspection.catalogRevision,
          organizationRevision: inspection.organizationRevision,
          selectionDigest: inspection.selectionDigest,
          groups: BigInt.from(inspection.groups),
          albumGroups: BigInt.from(inspection.albumGroups),
          favoriteGroups: BigInt.from(inspection.favoriteGroups),
          progressGroups: BigInt.from(inspection.progressGroups),
          classificationGroups: BigInt.from(inspection.classificationGroups),
          duplicateChoices: BigInt.from(inspection.duplicateChoices),
          oldestUnassociatedUnixMs: inspection.oldestUnassociatedUnixMs,
          newestUnassociatedUnixMs: inspection.newestUnassociatedUnixMs,
        ),
      ),
    ),
  );

  @override
  void cancel() => task.cancel();
  @override
  void dispose() => task.dispose();
}

Stream<LibraryCleanupEvent> _events(
  Stream<dto.OrganizationCleanupEvent> source,
) async* {
  try {
    await for (final event in source) {
      yield switch (event) {
        dto.OrganizationCleanupEvent_Progress(:final progress) =>
          LibraryCleanupProgress(switch (progress.phase) {
            dto.OrganizationCleanupPhase.inspecting =>
              LibraryCleanupPhase.inspecting,
            dto.OrganizationCleanupPhase.applying =>
              LibraryCleanupPhase.applying,
          }),
        dto.OrganizationCleanupEvent_Inspected(:final inspection) =>
          LibraryCleanupInspected(
            LibraryCleanupInspection(
              cutoffUnixMs: inspection.cutoffUnixMs,
              inspectedUnixMs: inspection.inspectedUnixMs,
              catalogRevision: inspection.catalogRevision,
              organizationRevision: inspection.organizationRevision,
              selectionDigest: inspection.selectionDigest,
              groups: inspection.groups.toInt(),
              albumGroups: inspection.albumGroups.toInt(),
              favoriteGroups: inspection.favoriteGroups.toInt(),
              progressGroups: inspection.progressGroups.toInt(),
              classificationGroups: inspection.classificationGroups.toInt(),
              duplicateChoices: inspection.duplicateChoices.toInt(),
              oldestUnassociatedUnixMs: inspection.oldestUnassociatedUnixMs,
              newestUnassociatedUnixMs: inspection.newestUnassociatedUnixMs,
            ),
          ),
        dto.OrganizationCleanupEvent_Completed(:final receipt) =>
          LibraryCleanupCompleted(
            receipt.operationId,
            receipt.removedRecords.toInt(),
          ),
        dto.OrganizationCleanupEvent_Cancelled() =>
          const LibraryCleanupCancelled(),
        dto.OrganizationCleanupEvent_Failed(:final code, :final message) =>
          throw LibraryCleanupFailure(code, message),
      };
    }
  } on rust.ScanError catch (error) {
    throw LibraryCleanupFailure(error.code, error.message);
  }
}
