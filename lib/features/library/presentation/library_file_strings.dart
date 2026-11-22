import "../../../app/presentation/ame_localizations.dart";
import "../application/library_file_operations.dart";
import "../domain/library_file_operations.dart";

abstract final class LibraryFileStrings {
  static String deletionTitle(
    AppLocalizations strings,
    LibraryFileSelectionSummary summary,
  ) {
    if (summary.cleanup != null) {
      return summary.files == 0
          ? strings.noPhotosToDelete
          : strings.aboutToDeletePhotos(summary.files);
    }
    final groups = summary.mergedGroups;
    if (groups == null || summary.files == groups) {
      return strings.confirmDeleteFiles(summary.files);
    }
    if (groups == 1) return strings.confirmDeleteWholeGroup(summary.files);
    return strings.confirmDeleteGroups(groups, summary.files);
  }

  static String rule(
    AppLocalizations strings,
    LibraryFileOrganizationRule rule,
  ) => switch (rule) {
    LibraryFileOrganizationRule.direct => strings.destinationFolderDirect,
    LibraryFileOrganizationRule.category => strings.byCategory,
    LibraryFileOrganizationRule.album => strings.byAlbum,
    LibraryFileOrganizationRule.date => strings.byDate,
    LibraryFileOrganizationRule.source => strings.bySourceFolder,
    LibraryFileOrganizationRule.albumYear => strings.byAlbumYear,
  };
  static String ruleDescription(
    AppLocalizations strings,
    LibraryFileOrganizationRule rule,
  ) => switch (rule) {
    LibraryFileOrganizationRule.date => strings.dateFolderRule,
    LibraryFileOrganizationRule.albumYear => strings.albumYearRule,
    _ => strings.unknownDestinationRule,
  };
  static String action(AppLocalizations strings, LibraryFileAction action) =>
      switch (action) {
        LibraryFileAction.copy => strings.copy,
        LibraryFileAction.move => strings.move,
        LibraryFileAction.rename => strings.rename,
        LibraryFileAction.recycle ||
        LibraryFileAction.deletePermanently => strings.deleteAction,
      };

  static String result(
    AppLocalizations strings,
    LibraryFileProgress progress,
    LibraryFileAction? action,
  ) {
    final deletion =
        action == LibraryFileAction.recycle ||
        action == LibraryFileAction.deletePermanently;
    final outcomes = [
      if (progress.published > 0)
        strings.filesCompleted(
          deletion ? strings.deleted : strings.completed,
          progress.published,
        ),
      if (progress.failed > 0) strings.filesIncomplete(progress.failed),
      if (progress.cancelled > 0) strings.filesStopped(progress.cancelled),
      if (progress.skipped > 0) strings.filesSkipped(progress.skipped),
    ];
    return outcomes.isEmpty
        ? strings.noFilesProcessed
        : strings.listSentence(outcomes.join(strings.clauseSeparator));
  }

  static String activity(
    AppLocalizations strings,
    LibraryFileActivity activity,
  ) {
    final progress = activity.progress!;
    if (activity.stopping) return strings.stopping;
    if (activity.purpose == LibraryFileActivityPurpose.report) {
      if (progress.phase == LibraryFileTaskPhase.finished) {
        return progress.export?.reportState == LibraryFileReportState.notNeeded
            ? strings.reportNotNeeded
            : strings.reportSaved;
      }
      if (progress.phase == LibraryFileTaskPhase.needsAttention) {
        return strings.reportFailed;
      }
    }
    final verb = switch (activity.purpose) {
      LibraryFileActivityPurpose.action => action(strings, activity.action!),
      LibraryFileActivityPurpose.recovery => strings.checkFileOperation,
      LibraryFileActivityPurpose.report => strings.generateReport,
    };
    final status = switch (progress.phase) {
      LibraryFileTaskPhase.preparing => strings.preparingOperation(verb),
      LibraryFileTaskPhase.running ||
      LibraryFileTaskPhase.refreshing => strings.runningOperation(verb),
      LibraryFileTaskPhase.stopping => strings.stopping,
      LibraryFileTaskPhase.finished => strings.operationEnded(verb),
      LibraryFileTaskPhase.cancelled => strings.operationStoppedLabel(verb),
      LibraryFileTaskPhase.needsAttention =>
        strings.fileOperationNeedsAttention,
    };
    if (!progress.isFinished ||
        progress.failure != null ||
        activity.purpose == LibraryFileActivityPurpose.report) {
      return status;
    }
    final summary = result(strings, progress, activity.action);
    return activity.action == LibraryFileAction.recycle ||
            activity.action == LibraryFileAction.deletePermanently
        ? summary
        : [status, summary].join(strings.clauseSeparator);
  }

  static String phase(AppLocalizations strings, LibraryFileRecordPhase phase) =>
      switch (phase) {
        LibraryFileRecordPhase.prepared ||
        LibraryFileRecordPhase.nameOccupied => strings.notStarted,
        LibraryFileRecordPhase.executing ||
        LibraryFileRecordPhase.removingSource => strings.verifyResults,
        LibraryFileRecordPhase.copied => strings.moveIncomplete,
        LibraryFileRecordPhase.nativeCompleted =>
          strings.filesProcessedRefreshPending,
        LibraryFileRecordPhase.published => strings.completed,
        LibraryFileRecordPhase.failed => strings.incomplete,
        LibraryFileRecordPhase.cancelled => strings.stopped,
        LibraryFileRecordPhase.uncertain => strings.resultsToVerify,
      };

  static String failure(
    AppLocalizations strings,
    LibraryFileFailure? failure, {
    LibraryFileAction? action,
  }) => switch (failure?.code) {
    "organization_evidence_pending" => strings.duplicateVerificationPending,
    "organization_selection_changed" ||
    "file_operation_selection_changed" => strings.selectedPhotosChanged,
    "file_operation_source_changed" =>
      action == LibraryFileAction.recycle ||
              action == LibraryFileAction.deletePermanently
          ? strings.deleteChangedFile
          : strings.selectedFilesChanged,
    "file_operation_source_unverified" =>
      action == LibraryFileAction.recycle ||
              action == LibraryFileAction.deletePermanently
          ? strings.deleteFilesUnverified
          : strings.filesUnverified,
    "file_operation_locations_ambiguous" => strings.multipleFileLocations,
    "file_operation_same_destination" => strings.destinationMatchesSource,
    "file_operation_destination_unavailable" => strings.destinationUnavailable,
    "file_operation_source_unavailable" => strings.selectedFilesUnreadable,
    "file_operation_keep_unavailable" => strings.retainedPhotoChanged,
    "file_operation_name_invalid" => strings.fileNameUnavailable,
    "file_operation_preparation_failed" => strings.fileOperationPrepareFailed,
    "file_operation_busy" ||
    "file_operation_selection_busy" => strings.fileOperationBusy,
    "file_operation_inspection_required" => strings.fileHistoryCheckRequired,
    "file_operation_selection_closed" => strings.fileOperationCancelled,
    "file_export_report_failed" => strings.reportWriteFailed,
    _ => strings.fileProcessingUncertain,
  };
}
