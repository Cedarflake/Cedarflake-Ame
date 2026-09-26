import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_synchronization_models.dart";
import "library_strings.dart";

class LibrarySynchronizationFeedback {
  factory LibrarySynchronizationFeedback.fromStatus(
    LibraryRootSynchronizationStatus status,
  ) {
    return LibrarySynchronizationFeedback._(
      message: _synchronizationNotificationMessage(status),
      detail: _synchronizationNotificationDetail(status),
      severity: _synchronizationNotificationSeverity(status),
      requiresManualLibraryUpdate: _requiresManualLibraryUpdate(status),
    );
  }

  const LibrarySynchronizationFeedback._({
    required this.message,
    required this.detail,
    required this.severity,
    required this.requiresManualLibraryUpdate,
  });

  final String message;
  final String? detail;
  final AmeNotificationSeverity severity;
  final bool requiresManualLibraryUpdate;
}

bool _requiresManualLibraryUpdate(LibraryRootSynchronizationStatus status) {
  return status.lastIssueCode == "legacy_recovery_authority_missing" ||
      (status.recoveryBlocked &&
          status.lastIssueCode == "live_gap_v30_explicit_recovery_required");
}

String _synchronizationNotificationMessage(
  LibraryRootSynchronizationStatus status,
) {
  if (status.isAwaitingFirstImport) {
    return LibraryStrings.firstImportRequiredDetail;
  }
  if (status.lastIssueCode == "legacy_recovery_authority_missing") {
    return LibraryStrings.synchronizationLegacyRecoveryAuthorityMissing;
  }
  if (status.lastIssueCode == "live_gap_v30_explicit_recovery_required") {
    return LibraryStrings.synchronizationExplicitRecoveryRequired;
  }
  if (status.recoveryBlocked) {
    return LibraryStrings.synchronizationRecoveryBlocked;
  }
  final issueMessage = _synchronizationIssueMessage(status.lastIssueCode);
  if (issueMessage != null) {
    return issueMessage;
  }
  return switch (status.freshnessCause) {
    LibraryCatalogFreshnessCause.changeSourceUnhealthy =>
      LibraryStrings.synchronizationSourceUnhealthy,
    LibraryCatalogFreshnessCause.evidenceGap =>
      LibraryStrings.synchronizationEvidenceGap,
    LibraryCatalogFreshnessCause.boundedCapacityExceeded =>
      LibraryStrings.synchronizationCapacityExceeded,
    LibraryCatalogFreshnessCause.noPendingChanges ||
    LibraryCatalogFreshnessCause.pendingChanges ||
    LibraryCatalogFreshnessCause.rootUnavailable =>
      LibraryStrings.synchronizationNeedsReconciliation,
  };
}

String? _synchronizationIssueMessage(String? issueCode) {
  final message = switch (issueCode) {
    "change_source_callback_access_denied" ||
    "change_source_start_access_denied" ||
    "change_source_watch_access_denied" =>
      LibraryStrings.synchronizationMonitoringAccessDenied,
    "change_source_callback_path_unavailable" ||
    "change_source_root_removed" ||
    "change_source_root_unavailable" =>
      LibraryStrings.synchronizationMonitoringPathUnavailable,
    "change_source_callback_capacity_exceeded" ||
    "change_source_ingress_overflow" ||
    "change_source_rescan_required" ||
    "change_source_start_capacity_exceeded" ||
    "change_source_watch_capacity_exceeded" =>
      LibraryStrings.synchronizationMonitoringCapacityExceeded,
    "change_source_event_incomplete" || "change_source_rename_incomplete" =>
      LibraryStrings.synchronizationMonitoringEventIncomplete,
    "change_source_callback_failed" ||
    "change_source_callback_invalid_configuration" ||
    "change_source_callback_io_failed" ||
    "change_source_callback_watch_missing" ||
    "change_source_ingress_disconnected" ||
    "change_source_state_unavailable" ||
    "change_source_start_failed" ||
    "change_source_start_invalid_configuration" ||
    "change_source_watch_invalid_configuration" ||
    "change_source_watch_failed" =>
      LibraryStrings.synchronizationMonitoringFailed,
    _ => null,
  };
  if (message != null || issueCode == null) {
    return message;
  }
  if (issueCode.startsWith("authoritative_") ||
      issueCode.startsWith("library_reconciliation_")) {
    return LibraryStrings.synchronizationRecoveryFailed;
  }
  if (issueCode.startsWith("catalog_") ||
      issueCode.startsWith("change_queue_") ||
      issueCode.startsWith("database_")) {
    return LibraryStrings.synchronizationPersistenceFailed;
  }
  return null;
}

String? _synchronizationNotificationDetail(
  LibraryRootSynchronizationStatus status,
) {
  if (status.isAwaitingFirstImport) {
    return LibraryStrings.firstImportAwaitingUser;
  }
  final details = <String>["阶段：${_synchronizationPhaseLabel(status.phase)}"];
  if (status.pendingChangeCount > BigInt.zero) {
    details.add("${status.pendingChangeCount} 项等待处理");
  }
  if (status.retryWaitCount > BigInt.zero) {
    // The total includes exhausted work; it does not establish a future retry.
    if (status.recoveryBlocked) {
      details.add("${status.retryWaitCount} 项更新尚未完成");
    } else {
      details.add("${status.retryWaitCount} 项等待重试");
    }
  }
  if (status.freshnessUnknownCount > BigInt.zero) {
    details.add("${status.freshnessUnknownCount} 项状态尚未确认");
  }
  return details.join(" · ");
}

String _synchronizationPhaseLabel(LibrarySynchronizationPhase phase) {
  return switch (phase) {
    LibrarySynchronizationPhase.watcherStartup =>
      LibraryStrings.synchronizationPhaseWatcherStartup,
    LibrarySynchronizationPhase.inventoryEnumeration =>
      LibraryStrings.synchronizationPhaseInventoryEnumeration,
    LibrarySynchronizationPhase.inventoryComparison =>
      LibraryStrings.synchronizationPhaseInventoryComparison,
    LibrarySynchronizationPhase.queuePublication =>
      LibraryStrings.synchronizationPhaseQueuePublication,
    LibrarySynchronizationPhase.retryWait =>
      LibraryStrings.synchronizationPhaseRetryWait,
    LibrarySynchronizationPhase.reconciliation =>
      LibraryStrings.synchronizationPhaseReconciliation,
    LibrarySynchronizationPhase.fullScan =>
      LibraryStrings.synchronizationPhaseFullScan,
    LibrarySynchronizationPhase.blocked =>
      LibraryStrings.synchronizationPhaseBlocked,
    LibrarySynchronizationPhase.synchronized =>
      LibraryStrings.synchronizationPhaseSynchronized,
    LibrarySynchronizationPhase.unavailable =>
      LibraryStrings.synchronizationPhaseUnavailable,
  };
}

AmeNotificationSeverity _synchronizationNotificationSeverity(
  LibraryRootSynchronizationStatus status,
) {
  if (status.isAwaitingFirstImport) {
    return AmeNotificationSeverity.info;
  }
  final issueCode = status.lastIssueCode;
  if (status.sourceStatus == LibraryChangeSourceStatus.failed ||
      issueCode?.startsWith("catalog_") == true ||
      issueCode?.startsWith("change_queue_") == true ||
      issueCode?.startsWith("database_") == true) {
    return AmeNotificationSeverity.error;
  }
  return AmeNotificationSeverity.warning;
}
