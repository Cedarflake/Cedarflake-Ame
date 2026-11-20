import "../../../app/presentation/ame_localizations.dart";
import "../../library/application/library_archive_requests.dart";
import "../../library/application/library_archive_state.dart";
import "../../library/domain/library_organization_archive.dart";

String archiveOperationName(
  AppLocalizations strings,
  LibraryArchiveRequest request,
) => switch (request) {
  LibraryArchiveExport() => strings.exportAction,
  LibraryArchiveInspect() => strings.inspect,
  LibraryArchiveRestore() => strings.restore,
};

String archiveProgressMessage(
  AppLocalizations strings,
  LibraryArchiveWorking state,
) {
  if (state.stopping) return strings.stopping;
  return switch (state.phase) {
    LibraryArchivePhase.snapshot => strings.preparingBackup,
    LibraryArchivePhase.writing => strings.exportingArchive,
    LibraryArchivePhase.verifying => strings.checkingBackup,
    LibraryArchivePhase.matching => strings.linkingPhotos,
    LibraryArchivePhase.restoring => strings.restoringArchive,
    null => strings.archiveInProgress(
      archiveOperationName(strings, state.request),
    ),
  };
}

String archiveFailureMessage(
  AppLocalizations strings,
  LibraryArchiveFailure failure,
) => switch (failure.code) {
  "organization_archive_destination_protected" => strings.archiveOutsideLibrary,
  "organization_archive_destination_exists" => strings.archiveFileExists,
  "organization_archive_version_unsupported" =>
    strings.archiveVersionUnsupported,
  "organization_archive_invalid" ||
  "organization_archive_records_invalid" => strings.archiveInvalid,
  "organization_archive_inspection_changed" => strings.archiveLibraryChanged,
  "organization_archive_input_changed" => strings.archiveChanged,
  "organization_archive_busy" => strings.archiveAlreadyRunning,
  "organization_archive_input_not_local" => strings.archiveUnreadable,
  "organization_archive_io" => strings.archiveIoFailed,
  "organization_archive_identity_conflict" ||
  "organization_archive_history_conflict" ||
  "organization_archive_operation_conflict" =>
    strings.archiveConflictsWithCurrent,
  _ => strings.archiveResultUnknown,
};

String archiveFilename(String path) => path.split(RegExp(r"[\\/]")).last;
