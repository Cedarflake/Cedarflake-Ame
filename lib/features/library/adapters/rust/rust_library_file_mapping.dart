import "../../../../src/rust/domain.dart" as rust;
import "../../../../src/rust/domain/file_operation_journal.dart" as journal;
import "../../../../src/rust/domain/file_operation_exports.dart" as exports;
import "../../../../src/rust/domain/file_operation_task.dart" as task;
import "../../../../src/rust/domain/file_operations.dart" as native;
import "../../domain/library_file_operations.dart";
import "../../domain/library_organization.dart";

native.FileAction fileActionToRust(LibraryFileAction action) =>
    switch (action) {
      LibraryFileAction.copy => native.FileAction.copy,
      LibraryFileAction.move => native.FileAction.move,
      LibraryFileAction.rename => native.FileAction.rename,
      LibraryFileAction.recycle => native.FileAction.recycle,
      LibraryFileAction.deletePermanently =>
        native.FileAction.deletePermanently,
    };

LibraryFileAction fileActionFromRust(native.FileAction action) =>
    switch (action) {
      native.FileAction.copy => LibraryFileAction.copy,
      native.FileAction.move => LibraryFileAction.move,
      native.FileAction.rename => LibraryFileAction.rename,
      native.FileAction.recycle => LibraryFileAction.recycle,
      native.FileAction.deletePermanently =>
        LibraryFileAction.deletePermanently,
    };

LibraryFileProgress fileProgressFromRust(
  task.FileOperationTaskProgress value,
) => LibraryFileProgress(
  phase: switch (value.phase) {
    task.FileOperationTaskPhase.starting => LibraryFileTaskPhase.preparing,
    task.FileOperationTaskPhase.running => LibraryFileTaskPhase.running,
    task.FileOperationTaskPhase.stopping => LibraryFileTaskPhase.stopping,
    task.FileOperationTaskPhase.finished => LibraryFileTaskPhase.finished,
    task.FileOperationTaskPhase.cancelled => LibraryFileTaskPhase.cancelled,
    task.FileOperationTaskPhase.needsAttention =>
      LibraryFileTaskPhase.needsAttention,
  },
  total: value.totalItems.toInt(),
  processed: value.processedItems.toInt(),
  published: value.publishedItems.toInt(),
  failed: value.failedItems.toInt(),
  cancelled: value.cancelledItems.toInt(),
  skipped: value.skippedItems.toInt(),
  export: value.export_ == null ? null : fileExportFromRust(value.export_!),
  path: value.currentPath,
  copiedBytes: value.copiedBytes.toInt(),
  totalBytes: value.totalBytes.toInt(),
  failure: value.failure == null
      ? null
      : LibraryFileFailure(value.failure!.code, value.failure!.message),
);

exports.FileOrganizationRule fileOrganizationRuleToRust(
  LibraryFileOrganizationRule rule,
) => switch (rule) {
  LibraryFileOrganizationRule.direct => exports.FileOrganizationRule.direct,
  LibraryFileOrganizationRule.category => exports.FileOrganizationRule.category,
  LibraryFileOrganizationRule.album => exports.FileOrganizationRule.album,
  LibraryFileOrganizationRule.date => exports.FileOrganizationRule.date,
  LibraryFileOrganizationRule.source => exports.FileOrganizationRule.source,
  LibraryFileOrganizationRule.albumYear =>
    exports.FileOrganizationRule.albumYear,
};

LibraryFileExport fileExportFromRust(exports.FileExportRecord value) =>
    LibraryFileExport(
      id: value.id,
      batchId: value.batch.batchId,
      directory: value.batch.root.path,
      selected: value.batch.selectedFiles.toInt(),
      skipped: value.skippedFiles.toInt(),
      createdAt: DateTime.fromMillisecondsSinceEpoch(
        value.batch.authorizedUnixMs.toInt(),
      ),
      reportState: switch (value.report) {
        exports.FileExportReport_Pending() => LibraryFileReportState.pending,
        exports.FileExportReport_NotNeeded() =>
          LibraryFileReportState.notNeeded,
        exports.FileExportReport_Written() => LibraryFileReportState.written,
        exports.FileExportReport_Failed() => LibraryFileReportState.failed,
      },
      reportPath: switch (value.report) {
        exports.FileExportReport_Written(:final field0) => field0,
        _ => null,
      },
      failure: switch (value.report) {
        exports.FileExportReport_Failed(:final field0) => LibraryFileFailure(
          field0.code,
          field0.message,
        ),
        _ => null,
      },
    );

LibraryFileRecord fileRecordFromRust(journal.FileOperationRecord value) {
  final head = value.checkpoint;
  final target = head.destination;
  final changedDeletion =
      {
        native.FileAction.recycle,
        native.FileAction.deletePermanently,
      }.contains(value.authorization.request.action) &&
      head.issue?.kind == native.FileOperationIssueKind.sourceChanged;
  return LibraryFileRecord(
    id: value.id,
    action: fileActionFromRust(value.authorization.request.action),
    source: value.authorization.request.source.state.absolutePath,
    destination:
        head.receipt?.destination?.state.absolutePath ??
        head.copied?.state.absolutePath ??
        (target == null ? null : "${target.directory.path}\\${target.name}"),
    phase: switch (head.phase) {
      journal.FileOperationPhase.prepared => LibraryFileRecordPhase.prepared,
      journal.FileOperationPhase.executing => LibraryFileRecordPhase.executing,
      journal.FileOperationPhase.nameOccupied =>
        LibraryFileRecordPhase.nameOccupied,
      journal.FileOperationPhase.copied => LibraryFileRecordPhase.copied,
      journal.FileOperationPhase.removingSource =>
        LibraryFileRecordPhase.removingSource,
      journal.FileOperationPhase.nativeCompleted =>
        LibraryFileRecordPhase.nativeCompleted,
      journal.FileOperationPhase.published => LibraryFileRecordPhase.published,
      journal.FileOperationPhase.failed => LibraryFileRecordPhase.failed,
      journal.FileOperationPhase.cancelled => LibraryFileRecordPhase.cancelled,
      journal.FileOperationPhase.uncertain => LibraryFileRecordPhase.uncertain,
    },
    updatedAt: DateTime.fromMillisecondsSinceEpoch(value.updatedUnixMs.toInt()),
    canContinueMove:
        head.copied != null &&
        {
          journal.FileOperationPhase.copied,
          journal.FileOperationPhase.failed,
          journal.FileOperationPhase.cancelled,
        }.contains(head.phase),
    canOpenRecycleBin:
        head.receipt?.recovery == native.FileOperationRecovery.systemRecycleBin,
    target: LibraryOrganizationTarget(
      assetId: value.authorization.catalog.assetId,
      locationId: value.authorization.catalog.locationId,
      sourceGeneration: value.authorization.catalog.sourceGeneration,
    ),
    canRetry:
        head.copied == null &&
        !changedDeletion &&
        {
          journal.FileOperationPhase.failed,
          journal.FileOperationPhase.cancelled,
        }.contains(head.phase),
    detail: head.issue?.message,
    failure: changedDeletion
        ? LibraryFileFailure(
            "file_operation_source_unverified",
            head.issue!.message,
          )
        : null,
  );
}

Future<T> fileOperationCall<T>(Future<T> Function() action) async {
  try {
    return await action();
  } on rust.ScanError catch (error) {
    throw LibraryFileFailure(error.code, error.message);
  }
}
