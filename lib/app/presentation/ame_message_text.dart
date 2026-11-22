import "../messages/ame_message.dart";
import "ame_localizations.dart";

extension AmeMessageText on AmeMessage {
  String resolve(AppLocalizations strings) => switch (this) {
    AmeRawMessage(:final value) => value,
    AmeDownloadProgressMessage(
      :final percent,
      :final receivedMegabytes,
      :final totalMegabytes,
    ) =>
      strings.modelDownloadProgress(percent, receivedMegabytes, totalMegabytes),
    AmeMessageList(:final messages) =>
      messages.map((message) => message.resolve(strings)).join(" · "),
    AmeRootStatusMessage(:final name, :final status) => strings.folderStatus(
      name,
      status.resolve(strings),
    ),
    final AmeLabel label => switch (label) {
      AmeLabel.catalogCleanupAlreadyFinished =>
        strings.catalogCleanupAlreadyFinished,
      AmeLabel.albumDetailsUnavailable => strings.albumDetailsUnavailable,
      AmeLabel.acknowledge => strings.acknowledge,
      AmeLabel.actionFailed => strings.actionFailed,
      AmeLabel.addOrMoveToAlbum => strings.addOrMoveToAlbum,
      AmeLabel.addToAlbum => strings.addToAlbum,
      AmeLabel.albumConflict => strings.albumConflict,
      AmeLabel.albumCreated => strings.albumCreated,
      AmeLabel.albumDeleted => strings.albumDeleted,
      AmeLabel.albumIconChanged => strings.albumIconChanged,
      AmeLabel.albumMissing => strings.albumMissing,
      AmeLabel.albumNameInvalid => strings.albumNameInvalid,
      AmeLabel.albumNameTaken => strings.albumNameTaken,
      AmeLabel.albumRemovalRemaining => strings.albumRemovalRemaining,
      AmeLabel.albumRenamed => strings.albumRenamed,
      AmeLabel.albumsAndFavoritesUnavailable =>
        strings.albumsAndFavoritesUnavailable,
      AmeLabel.archiveExported => strings.archiveExported,
      AmeLabel.archiveRestored => strings.archiveRestored,
      AmeLabel.backgroundVerification => strings.backgroundVerification,
      AmeLabel.backupSaved => strings.backupSaved,
      AmeLabel.cancel => strings.cancel,
      AmeLabel.changePhotoFailed => strings.changePhotoFailed,
      AmeLabel.changesSaveFailed => strings.changesSaveFailed,
      AmeLabel.changesSaved => strings.changesSaved,
      AmeLabel.changesToCheck => strings.changesToCheck,
      AmeLabel.changesUnconfirmed => strings.changesUnconfirmed,
      AmeLabel.chooseConflictAlbum => strings.chooseConflictAlbum,
      AmeLabel.conflictOpenFailed => strings.conflictOpenFailed,
      AmeLabel.continuityLiveOnly => strings.continuityLiveOnly,
      AmeLabel.copyPathFailed => strings.copyPathFailed,
      AmeLabel.copyPhotoFailed => strings.copyPhotoFailed,
      AmeLabel.deferredOrganization => strings.deferredOrganization,
      AmeLabel.favoriteAdded => strings.favoriteAdded,
      AmeLabel.favoriteRemoved => strings.favoriteRemoved,
      AmeLabel.filePathCopied => strings.filePathCopied,
      AmeLabel.filePickerFailed => strings.filePickerFailed,
      AmeLabel.firstImportIncomplete => strings.firstImportIncomplete,
      AmeLabel.folderRemovalFailed => strings.folderRemovalFailed,
      AmeLabel.folderRemovedRefreshPending =>
        strings.folderRemovedRefreshPending,
      AmeLabel.folderStillUpdating => strings.folderStillUpdating,
      AmeLabel.galleryDisplayReloadPending =>
        strings.galleryDisplayReloadPending,
      AmeLabel.galleryReturnFailed => strings.galleryReturnFailed,
      AmeLabel.gpuSettingFailed => strings.gpuSettingFailed,
      AmeLabel.gpuSettingRetry => strings.gpuSettingRetry,
      AmeLabel.ignored => strings.ignored,
      AmeLabel.libraryRefreshPending => strings.libraryRefreshPending,
      AmeLabel.libraryUpdateBlocked => strings.libraryUpdateBlocked,
      AmeLabel.manualUpdateDetail => strings.manualUpdateDetail,
      AmeLabel.manualUpdateNotice => strings.manualUpdateNotice,
      AmeLabel.modelConfigUnreadable => strings.modelConfigUnreadable,
      AmeLabel.modelDownloadRetained => strings.modelDownloadRetained,
      AmeLabel.modelDownloading => strings.modelDownloading,
      AmeLabel.modelEnableCancelled => strings.modelEnableCancelled,
      AmeLabel.modelEnableFailed => strings.modelEnableFailed,
      AmeLabel.modelEnableRetry => strings.modelEnableRetry,
      AmeLabel.modelEnabled => strings.modelEnabled,
      AmeLabel.modelEnabling => strings.modelEnabling,
      AmeLabel.modelPreparing => strings.modelPreparing,
      AmeLabel.modelVerifying => strings.modelVerifying,
      AmeLabel.moveToAnotherAlbum => strings.moveToAnotherAlbum,
      AmeLabel.needsReconciliation => strings.needsReconciliation,
      AmeLabel.noActionablePhotos => strings.noActionablePhotos,
      AmeLabel.openInExplorerFailed => strings.openInExplorerFailed,
      AmeLabel.operationFailedDetails => strings.operationFailedDetails,
      AmeLabel.operationIncomplete => strings.operationIncomplete,
      AmeLabel.operationStopped => strings.operationStopped,
      AmeLabel.organizationDataChanged => strings.organizationDataChanged,
      AmeLabel.organizationEnterFailed => strings.organizationEnterFailed,
      AmeLabel.organizationHistoryChanged => strings.organizationHistoryChanged,
      AmeLabel.organizationLibraryUpdating =>
        strings.organizationLibraryUpdating,
      AmeLabel.organizationNextFailed => strings.organizationNextFailed,
      AmeLabel.organizationPositionChanged =>
        strings.organizationPositionChanged,
      AmeLabel.organizationPositionUnreadable =>
        strings.organizationPositionUnreadable,
      AmeLabel.organizationPositionUnsaved =>
        strings.organizationPositionUnsaved,
      AmeLabel.organizationRecordChanged => strings.organizationRecordChanged,
      AmeLabel.organizationReselectPhotos => strings.organizationReselectPhotos,
      AmeLabel.organizationResumeFailed => strings.organizationResumeFailed,
      AmeLabel.organizationScopeFailed => strings.organizationScopeFailed,
      AmeLabel.organizationSelectionChanged =>
        strings.organizationSelectionChanged,
      AmeLabel.organizationSelectionExpired =>
        strings.organizationSelectionExpired,
      AmeLabel.organizationViewChanged => strings.organizationViewChanged,
      AmeLabel.partialResults => strings.partialResults,
      AmeLabel.photoAnalysisPaused => strings.photoAnalysisPaused,
      AmeLabel.photoCopied => strings.photoCopied,
      AmeLabel.photoInformationUpdating => strings.photoInformationUpdating,
      AmeLabel.photoTooLargeToCopy => strings.photoTooLargeToCopy,
      AmeLabel.previousOperationRunning => strings.previousOperationRunning,
      AmeLabel.redoCompleted => strings.redoCompleted,
      AmeLabel.redoFailed => strings.redoFailed,
      AmeLabel.removedFromAlbum => strings.removedFromAlbum,
      AmeLabel.reopenMenuRetry => strings.reopenMenuRetry,
      AmeLabel.restoredToInbox => strings.restoredToInbox,
      AmeLabel.retry => strings.retry,
      AmeLabel.retryAnalysis => strings.retryAnalysis,
      AmeLabel.retryLoading => strings.retryLoading,
      AmeLabel.retrySearch => strings.retrySearch,
      AmeLabel.saveGallerySettingsFailed => strings.saveGallerySettingsFailed,
      AmeLabel.saveResultUnknown => strings.saveResultUnknown,
      AmeLabel.saveSelectionFailed => strings.saveSelectionFailed,
      AmeLabel.saveSidebarFailed => strings.saveSidebarFailed,
      AmeLabel.searchAnalysisUnavailable => strings.searchAnalysisUnavailable,
      AmeLabel.searchDescriptionTooLong => strings.searchDescriptionTooLong,
      AmeLabel.searchFailedRetained => strings.searchFailedRetained,
      AmeLabel.searchIndexDamaged => strings.searchIndexDamaged,
      AmeLabel.searchLibraryChanged => strings.searchLibraryChanged,
      AmeLabel.searchNeedsModel => strings.searchNeedsModel,
      AmeLabel.searchPartialAnalysis => strings.searchPartialAnalysis,
      AmeLabel.searchPhotoChanged => strings.searchPhotoChanged,
      AmeLabel.searchStorageUnavailable => strings.searchStorageUnavailable,
      AmeLabel.searchUnavailable => strings.searchUnavailable,
      AmeLabel.seekDateFailed => strings.seekDateFailed,
      AmeLabel.settingsSaveFailed => strings.settingsSaveFailed,
      AmeLabel.smartAnalysisInterrupted => strings.smartAnalysisInterrupted,
      AmeLabel.smartResultsAvailable => strings.smartResultsAvailable,
      AmeLabel.sourceUnavailable => strings.sourceUnavailable,
      AmeLabel.stopping => strings.stopping,
      AmeLabel.synchronizationAutomaticRetry =>
        strings.synchronizationAutomaticRetry,
      AmeLabel.synchronizationCapacityExceeded =>
        strings.synchronizationCapacityExceeded,
      AmeLabel.synchronizationCheckingChanges =>
        strings.synchronizationCheckingChanges,
      AmeLabel.synchronizationCheckingFolder =>
        strings.synchronizationCheckingFolder,
      AmeLabel.synchronizationComplete => strings.synchronizationComplete,
      AmeLabel.synchronizationCompleted => strings.synchronizationCompleted,
      AmeLabel.synchronizationEvidenceGap => strings.synchronizationEvidenceGap,
      AmeLabel.synchronizationLegacyRecoveryAuthorityMissing =>
        strings.synchronizationLegacyRecoveryAuthorityMissing,
      AmeLabel.synchronizationMonitoringAccessDenied =>
        strings.synchronizationMonitoringAccessDenied,
      AmeLabel.synchronizationMonitoringEventIncomplete =>
        strings.synchronizationMonitoringEventIncomplete,
      AmeLabel.synchronizationNeedsReconciliation =>
        strings.synchronizationNeedsReconciliation,
      AmeLabel.synchronizationPersistenceFailed =>
        strings.synchronizationPersistenceFailed,
      AmeLabel.synchronizationPreparing => strings.synchronizationPreparing,
      AmeLabel.synchronizationRecoveryBlocked =>
        strings.synchronizationRecoveryBlocked,
      AmeLabel.synchronizationRecoveryFailed =>
        strings.synchronizationRecoveryFailed,
      AmeLabel.synchronizationRefreshFailureMessage =>
        strings.synchronizationRefreshFailureMessage,
      AmeLabel.synchronizationRetryLater => strings.synchronizationRetryLater,
      AmeLabel.synchronizationRetryPending =>
        strings.synchronizationRetryPending,
      AmeLabel.synchronizationScanningFolder =>
        strings.synchronizationScanningFolder,
      AmeLabel.synchronizationSourceStarting =>
        strings.synchronizationSourceStarting,
      AmeLabel.synchronizationSourceUnavailable =>
        strings.synchronizationSourceUnavailable,
      AmeLabel.synchronizationSourceUnhealthy =>
        strings.synchronizationSourceUnhealthy,
      AmeLabel.synchronizationUnconfirmed => strings.synchronizationUnconfirmed,
      AmeLabel.synchronizationUpdateIncomplete =>
        strings.synchronizationUpdateIncomplete,
      AmeLabel.synchronizationUpdatingLibrary =>
        strings.synchronizationUpdatingLibrary,
      AmeLabel.synchronizationWaitingFolder =>
        strings.synchronizationWaitingFolder,
      AmeLabel.synchronized => strings.synchronized,
      AmeLabel.synchronizing => strings.synchronizing,
      AmeLabel.tryAgain => strings.tryAgain,
      AmeLabel.undo => strings.undo,
      AmeLabel.undoCompleted => strings.undoCompleted,
      AmeLabel.undoFailed => strings.undoFailed,
      AmeLabel.unlinkedRecordsCleared => strings.unlinkedRecordsCleared,
      AmeLabel.unlinkedRecordsKept => strings.unlinkedRecordsKept,
      AmeLabel.updateLibrary => strings.updateLibrary,
      AmeLabel.viewAction => strings.viewAction,
    },
    AmeAddedToAlbumMessage(:final name) => strings.addedToAlbum(name),
    AmeAlbumRemovalFailedMessage(:final name) => strings.albumRemovalFailed(
      name,
    ),
    AmeConflictsDeferredMessage(:final count) => strings.conflictsDeferred(
      count,
    ),
    AmeFirstImportRequiredFolderMessage(:final name) =>
      strings.firstImportRequiredFolder(name),
    AmeIncompleteChangesMessage(:final count) => strings.incompleteChanges(
      count,
    ),
    AmeMovedToAlbumMessage(:final name) => strings.movedToAlbum(name),
    AmePendingChangesMessage(:final count) => strings.pendingChanges(count),
    AmeRecordsClearedMessage(:final count) => strings.recordsCleared(count),
    AmeRemovedFolderMessage(:final name) => strings.removedFolder(name),
    AmeRemovedFoldersMessage(:final count) => strings.removedFolders(count),
    AmeRetryingChangesMessage(:final count) => strings.retryingChanges(count),
    AmeSynchronizationBlockedFolderMessage(:final name) =>
      strings.synchronizationBlockedFolder(name),
    AmeSynchronizationUncheckedFolderMessage(:final name) =>
      strings.synchronizationUncheckedFolder(name),
  };
}
