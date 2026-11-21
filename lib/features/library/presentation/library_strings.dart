import "../../../app/messages/ame_message.dart";
import "../../../app/presentation/ame_localizations.dart";

abstract final class LibraryStrings {
  static const appName = "Cedarflake Ame";
  static const workbenchName = "Ame";

  static String? scanFailureDetail(AppLocalizations strings, String? error) {
    if (error == null) return null;
    final separator = error.indexOf(":");
    final code = separator < 0 ? error : error.substring(0, separator);
    return switch (code) {
      "source_root_overlaps_storage" => strings.scanOverlapsStorage,
      "library_update_in_progress" => strings.scanBusy,
      "library_retained_cancel_failed" => strings.scanCancelFailed,
      "catalog_root_consolidation_incomplete" => strings.parentScanIncomplete,
      "catalog_root_consolidation_busy" => strings.childScanPending,
      "catalog_root_consolidation_unpublished" => strings.childUpdateRequired,
      "catalog_root_consolidation_changed" ||
      "catalog_root_consolidation_identity_changed" => strings.childMergeFailed,
      "library_update_root_changed" => strings.scanFolderFailed,
      "catalog_source_view_superseded" => strings.locationRefreshFailed,
      "catalog_publication_view_superseded" => strings.displayRefreshSuperseded,
      "library_root_relocation_unavailable" => strings.relocationUnsupported,
      "scan_stream_ended" => strings.scanEndedWithoutResult,
      "library_update_paused" => strings.updatePausedRestart,
      "library_update_source_changed" => strings.sourceChangedDuringUpdate,
      "library_update_source_replaced" => strings.updateSourceChanged,
      "library_update_display_failed" => strings.libraryRefreshPending,
      _ => null,
    };
  }

  static AmeMessage synchronizationReconciliationTitle(String rootName) =>
      AmeSynchronizationBlockedFolderMessage(rootName);
  static AmeMessage synchronizationUnconfirmedTitle(String rootName) =>
      AmeSynchronizationUncheckedFolderMessage(rootName);
  static AmeMessage firstImportRequiredTitle(String rootName) =>
      AmeFirstImportRequiredFolderMessage(rootName);
}
