import "../../../app/messages/ame_message.dart";
import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_synchronization_models.dart";
import "library_strings.dart";
import "library_synchronization_feedback_state.dart";

/// 自动同步反馈：sourceLabel 用于侧栏；titleForRoot、message 和 detail 用于通知。
/// 是否发布通知由 shouldNotify 决定；当前仅受阻和中断状态产生目录通知。
class LibrarySynchronizationFeedback {
  factory LibrarySynchronizationFeedback.fromStatus(
    LibraryRootSynchronizationStatus status,
  ) {
    final state = LibrarySynchronizationFeedbackState.fromStatus(status);
    return LibrarySynchronizationFeedback._(
      state: state,
      sourceLabel: _synchronizationSourceLabel(status, state),
      message: _synchronizationNotificationMessage(status, state),
      detail: _synchronizationNotificationDetail(status, state),
      severity: _synchronizationNotificationSeverity(status, state),
      requiresManualLibraryUpdate:
          state == LibrarySynchronizationFeedbackState.blocked &&
          _requiresManualLibraryUpdate(status),
    );
  }

  const LibrarySynchronizationFeedback._({
    required this.state,
    required this.sourceLabel,
    required this.message,
    required this.detail,
    required this.severity,
    required this.requiresManualLibraryUpdate,
  });

  final LibrarySynchronizationFeedbackState state;
  final AmeMessage sourceLabel;
  final AmeMessage message;
  final AmeMessage? detail;
  final AmeNotificationSeverity severity;
  final bool requiresManualLibraryUpdate;

  bool get shouldNotify => state.shouldNotify;

  // 底部通知和通知详情｜带目录名称的标题；目录名称不可用时使用图库范围的标题。
  AmeMessage titleForRoot(String? rootName) {
    if (rootName == null) {
      return switch (state) {
        LibrarySynchronizationFeedbackState.blocked ||
        LibrarySynchronizationFeedbackState.interrupted =>
          AmeLabel.libraryUpdateBlocked,
        _ => sourceLabel,
      };
    }
    return switch (state) {
      LibrarySynchronizationFeedbackState.awaitingFirstImport =>
        LibraryStrings.firstImportRequiredTitle(rootName),
      LibrarySynchronizationFeedbackState.unconfirmed =>
        LibraryStrings.synchronizationUnconfirmedTitle(rootName),
      LibrarySynchronizationFeedbackState.updating ||
      LibrarySynchronizationFeedbackState.backgroundVerification ||
      LibrarySynchronizationFeedbackState.awaitingRetry ||
      LibrarySynchronizationFeedbackState.synchronized ||
      LibrarySynchronizationFeedbackState.unavailable => AmeRootStatusMessage(
        rootName,
        sourceLabel,
      ),
      LibrarySynchronizationFeedbackState.blocked ||
      LibrarySynchronizationFeedbackState.interrupted =>
        LibraryStrings.synchronizationReconciliationTitle(rootName),
    };
  }
}

bool _requiresManualLibraryUpdate(LibraryRootSynchronizationStatus status) {
  return status.lastIssueCode == "legacy_recovery_authority_missing" ||
      status.lastIssueCode == "live_gap_v30_explicit_recovery_required";
}

AmeMessage _synchronizationNotificationMessage(
  LibraryRootSynchronizationStatus status,
  LibrarySynchronizationFeedbackState state,
) {
  if (status.isAwaitingFirstImport) {
    return AmeLabel.firstImportIncomplete;
  }
  if (state == LibrarySynchronizationFeedbackState.unavailable) {
    return AmeLabel.synchronizationSourceUnavailable;
  }
  if (state == LibrarySynchronizationFeedbackState.blocked) {
    if (status.lastIssueCode == "legacy_recovery_authority_missing") {
      return AmeLabel.synchronizationLegacyRecoveryAuthorityMissing;
    }
    if (status.lastIssueCode == "live_gap_v30_explicit_recovery_required") {
      return AmeLabel.synchronizationLegacyRecoveryAuthorityMissing;
    }
    return AmeLabel.synchronizationRecoveryBlocked;
  }
  if (state == LibrarySynchronizationFeedbackState.awaitingRetry) {
    return AmeLabel.synchronizationAutomaticRetry;
  }
  if (state == LibrarySynchronizationFeedbackState.updating) {
    return _synchronizationPhaseLabel(status.phase);
  }
  if (state == LibrarySynchronizationFeedbackState.backgroundVerification) {
    return const AmeRawMessage("");
  }
  if (state == LibrarySynchronizationFeedbackState.synchronized) {
    if (status.continuity == LibraryContinuityState.liveOnly &&
        !status.usesAutomaticMetadataFallback) {
      return AmeLabel.manualUpdateDetail;
    }
    return AmeLabel.synchronizationCompleted;
  }
  final issueMessage = _synchronizationIssueMessage(status.lastIssueCode);
  if (issueMessage != null) {
    return issueMessage;
  }
  return switch (status.freshnessCause) {
    LibraryCatalogFreshnessCause.changeSourceUnhealthy =>
      status.sourceStatus == LibraryChangeSourceStatus.starting
          ? AmeLabel.synchronizationSourceStarting
          : AmeLabel.synchronizationSourceUnhealthy,
    LibraryCatalogFreshnessCause.evidenceGap =>
      AmeLabel.synchronizationEvidenceGap,
    LibraryCatalogFreshnessCause.boundedCapacityExceeded =>
      AmeLabel.synchronizationCapacityExceeded,
    LibraryCatalogFreshnessCause.noPendingChanges ||
    LibraryCatalogFreshnessCause.pendingChanges ||
    LibraryCatalogFreshnessCause.rootUnavailable =>
      AmeLabel.synchronizationNeedsReconciliation,
  };
}

AmeMessage? _synchronizationIssueMessage(String? issueCode) {
  final message = switch (issueCode) {
    "change_source_callback_access_denied" ||
    "change_source_start_access_denied" ||
    "change_source_watch_access_denied" =>
      AmeLabel.synchronizationMonitoringAccessDenied,
    "change_source_callback_path_unavailable" ||
    "change_source_root_removed" ||
    "change_source_root_unavailable" =>
      AmeLabel.synchronizationSourceUnavailable,
    "change_source_callback_capacity_exceeded" ||
    "change_source_ingress_overflow" ||
    "change_source_rescan_required" ||
    "change_source_start_capacity_exceeded" ||
    "change_source_watch_capacity_exceeded" =>
      AmeLabel.synchronizationCapacityExceeded,
    "change_source_event_incomplete" || "change_source_rename_incomplete" =>
      AmeLabel.synchronizationMonitoringEventIncomplete,
    "change_source_callback_failed" ||
    "change_source_callback_invalid_configuration" ||
    "change_source_callback_io_failed" ||
    "change_source_callback_watch_missing" ||
    "change_source_ingress_disconnected" ||
    "change_source_state_unavailable" ||
    "change_source_start_failed" ||
    "change_source_start_invalid_configuration" ||
    "change_source_watch_invalid_configuration" ||
    "change_source_watch_failed" => AmeLabel.synchronizationSourceUnhealthy,
    _ => null,
  };
  if (message != null || issueCode == null) {
    return message;
  }
  if (issueCode.startsWith("authoritative_") ||
      issueCode.startsWith("library_reconciliation_")) {
    return AmeLabel.synchronizationRecoveryFailed;
  }
  if (issueCode.startsWith("catalog_") ||
      issueCode.startsWith("change_queue_") ||
      issueCode.startsWith("database_")) {
    return AmeLabel.synchronizationPersistenceFailed;
  }
  return null;
}

// 通知补充说明｜等待处理、等待重试或尚未完成的项目数量。
AmeMessage? _synchronizationNotificationDetail(
  LibraryRootSynchronizationStatus status,
  LibrarySynchronizationFeedbackState state,
) {
  if (status.isAwaitingFirstImport) {
    return null;
  }
  final details = <AmeMessage>[];
  if (status.pendingChangeCount > BigInt.zero) {
    details.add(AmePendingChangesMessage(status.pendingChangeCount.toString()));
  }
  if (status.retryWaitCount > BigInt.zero) {
    // The total includes exhausted work; it does not establish a future retry.
    if (state == LibrarySynchronizationFeedbackState.awaitingRetry) {
      details.add(AmeRetryingChangesMessage(status.retryWaitCount.toString()));
    } else {
      details.add(
        AmeIncompleteChangesMessage(status.retryWaitCount.toString()),
      );
    }
  }
  if (status.freshnessUnknownCount > BigInt.zero &&
      state == LibrarySynchronizationFeedbackState.unconfirmed) {
    details.add(AmeLabel.changesToCheck);
  }
  return details.isEmpty ? null : AmeMessageList(details);
}

AmeMessage _synchronizationPhaseLabel(LibrarySynchronizationPhase phase) {
  return switch (phase) {
    LibrarySynchronizationPhase.watcherStartup =>
      AmeLabel.synchronizationPreparing,
    LibrarySynchronizationPhase.inventoryEnumeration =>
      AmeLabel.synchronizationCheckingFolder,
    LibrarySynchronizationPhase.inventoryComparison =>
      AmeLabel.synchronizationCheckingChanges,
    LibrarySynchronizationPhase.queuePublication =>
      AmeLabel.synchronizationUpdatingLibrary,
    LibrarySynchronizationPhase.retryWait => AmeLabel.synchronizationRetryLater,
    LibrarySynchronizationPhase.reconciliation =>
      AmeLabel.synchronizationCheckingChanges,
    LibrarySynchronizationPhase.fullScan =>
      AmeLabel.synchronizationScanningFolder,
    LibrarySynchronizationPhase.blocked =>
      AmeLabel.synchronizationUpdateIncomplete,
    LibrarySynchronizationPhase.synchronized =>
      AmeLabel.synchronizationComplete,
    LibrarySynchronizationPhase.unavailable =>
      AmeLabel.synchronizationWaitingFolder,
    LibrarySynchronizationPhase.backgroundVerification => const AmeRawMessage(
      "",
    ),
  };
}

AmeNotificationSeverity _synchronizationNotificationSeverity(
  LibraryRootSynchronizationStatus status,
  LibrarySynchronizationFeedbackState state,
) {
  if (state == LibrarySynchronizationFeedbackState.awaitingFirstImport ||
      state == LibrarySynchronizationFeedbackState.updating ||
      state == LibrarySynchronizationFeedbackState.backgroundVerification ||
      state == LibrarySynchronizationFeedbackState.awaitingRetry ||
      state == LibrarySynchronizationFeedbackState.unconfirmed ||
      state == LibrarySynchronizationFeedbackState.synchronized) {
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

AmeMessage _synchronizationSourceLabel(
  LibraryRootSynchronizationStatus status,
  LibrarySynchronizationFeedbackState state,
) => switch (state) {
  LibrarySynchronizationFeedbackState.awaitingFirstImport =>
    AmeLabel.firstImportIncomplete,
  LibrarySynchronizationFeedbackState.unavailable => AmeLabel.sourceUnavailable,
  LibrarySynchronizationFeedbackState.blocked ||
  LibrarySynchronizationFeedbackState.interrupted =>
    AmeLabel.needsReconciliation,
  LibrarySynchronizationFeedbackState.awaitingRetry =>
    AmeLabel.synchronizationRetryPending,
  LibrarySynchronizationFeedbackState.updating => AmeLabel.synchronizing,
  LibrarySynchronizationFeedbackState.backgroundVerification =>
    AmeLabel.backgroundVerification,
  LibrarySynchronizationFeedbackState.unconfirmed =>
    AmeLabel.synchronizationUnconfirmed,
  LibrarySynchronizationFeedbackState.synchronized =>
    status.continuity == LibraryContinuityState.liveOnly &&
            !status.usesAutomaticMetadataFallback
        ? AmeLabel.continuityLiveOnly
        : AmeLabel.synchronized,
};
