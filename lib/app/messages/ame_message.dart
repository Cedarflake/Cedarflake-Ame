/// Language-independent text retained by a task or notification.
sealed class AmeMessage {
  const AmeMessage();
}

enum AmeLabel implements AmeMessage {
  updateTitle,
  updateAvailable,
  updateDownloading,
  updateVerifying,
  updateStopping,
  updateReady,
  updateView,
  updateFailed,
  updateInstalled,
  updateCancelled,
  updateNetworkFailure,
  updateReleaseFailure,
  updatePackageFailure,
  updateStorageFailure,
  updateInstallationFailure,
  updateShutdownFailure,
  updateUnavailable,
  catalogCleanupAlreadyFinished,
  albumDetailsUnavailable,
  acknowledge,
  actionFailed,
  addOrMoveToAlbum,
  addToAlbum,
  albumConflict,
  albumCreated,
  albumDeleted,
  albumIconChanged,
  albumMissing,
  albumNameInvalid,
  albumNameTaken,
  albumRemovalRemaining,
  albumRenamed,
  albumsAndFavoritesUnavailable,
  archiveExported,
  archiveRestored,
  backgroundVerification,
  backupSaved,
  cancel,
  changePhotoFailed,
  changesSaveFailed,
  changesSaved,
  changesToCheck,
  changesUnconfirmed,
  chooseConflictAlbum,
  conflictOpenFailed,
  continuityLiveOnly,
  copyPathFailed,
  copyPhotoFailed,
  deferredOrganization,
  favoriteAdded,
  favoriteRemoved,
  filePathCopied,
  filePickerFailed,
  firstImportIncomplete,
  folderRemovalFailed,
  folderRemovedRefreshPending,
  folderStillUpdating,
  galleryDisplayReloadPending,
  galleryReturnFailed,
  gpuSettingFailed,
  gpuSettingRetry,
  ignored,
  libraryRefreshPending,
  libraryUpdateBlocked,
  manualUpdateDetail,
  manualUpdateNotice,
  modelConfigUnreadable,
  modelDownloadRetained,
  modelDownloading,
  modelEnableCancelled,
  modelEnableFailed,
  modelEnableRetry,
  modelEnabled,
  modelEnabling,
  modelPreparing,
  modelVerifying,
  moveToAnotherAlbum,
  needsReconciliation,
  noActionablePhotos,
  openInExplorerFailed,
  operationFailedDetails,
  operationIncomplete,
  operationStopped,
  organizationDataChanged,
  organizationEnterFailed,
  organizationHistoryChanged,
  organizationLibraryUpdating,
  organizationNextFailed,
  organizationPositionChanged,
  organizationPositionUnreadable,
  organizationPositionUnsaved,
  organizationRecordChanged,
  organizationReselectPhotos,
  organizationResumeFailed,
  organizationScopeFailed,
  organizationSelectionChanged,
  organizationSelectionExpired,
  organizationViewChanged,
  partialResults,
  photoAnalysisPaused,
  photoCopied,
  photoInformationUpdating,
  photoTooLargeToCopy,
  previousOperationRunning,
  redoCompleted,
  redoFailed,
  removedFromAlbum,
  reopenMenuRetry,
  restoredToInbox,
  retry,
  retryAnalysis,
  retryLoading,
  retrySearch,
  saveGallerySettingsFailed,
  saveResultUnknown,
  saveSelectionFailed,
  saveSidebarFailed,
  searchAnalysisUnavailable,
  searchDescriptionTooLong,
  searchFailedRetained,
  searchIndexDamaged,
  searchLibraryChanged,
  searchNeedsModel,
  searchPartialAnalysis,
  searchPhotoChanged,
  searchStorageUnavailable,
  searchUnavailable,
  seekDateFailed,
  settingsSaveFailed,
  smartAnalysisInterrupted,
  smartResultsAvailable,
  sourceUnavailable,
  stopping,
  synchronizationAutomaticRetry,
  synchronizationCapacityExceeded,
  synchronizationCheckingChanges,
  synchronizationCheckingFolder,
  synchronizationComplete,
  synchronizationCompleted,
  synchronizationEvidenceGap,
  synchronizationLegacyRecoveryAuthorityMissing,
  synchronizationMonitoringAccessDenied,
  synchronizationMonitoringEventIncomplete,
  synchronizationNeedsReconciliation,
  synchronizationPersistenceFailed,
  synchronizationPreparing,
  synchronizationRecoveryBlocked,
  synchronizationRecoveryFailed,
  synchronizationRefreshFailureMessage,
  synchronizationRetryLater,
  synchronizationRetryPending,
  synchronizationScanningFolder,
  synchronizationSourceStarting,
  synchronizationSourceUnavailable,
  synchronizationSourceUnhealthy,
  synchronizationUnconfirmed,
  synchronizationUpdateIncomplete,
  synchronizationUpdatingLibrary,
  synchronizationWaitingFolder,
  synchronized,
  synchronizing,
  tryAgain,
  undo,
  undoCompleted,
  undoFailed,
  unlinkedRecordsCleared,
  unlinkedRecordsKept,
  updateLibrary,
  viewAction,
}

final class AmeRawMessage extends AmeMessage {
  const AmeRawMessage(this.value);
  final String value;
}

final class AmeDownloadProgressMessage extends AmeMessage {
  const AmeDownloadProgressMessage(
    this.percent,
    this.receivedMegabytes,
    this.totalMegabytes,
  );
  final int percent;
  final double receivedMegabytes;
  final double totalMegabytes;
}

final class AmeMessageList extends AmeMessage {
  AmeMessageList(Iterable<AmeMessage> messages)
    : messages = List.unmodifiable(messages);
  final List<AmeMessage> messages;
}

final class AmeRootStatusMessage extends AmeMessage {
  const AmeRootStatusMessage(this.name, this.status);
  final String name;
  final AmeMessage status;
}

final class AmeAddedToAlbumMessage extends AmeMessage {
  const AmeAddedToAlbumMessage(this.name);
  final String name;
}

final class AmeAlbumRemovalFailedMessage extends AmeMessage {
  const AmeAlbumRemovalFailedMessage(this.name);
  final String name;
}

final class AmeConflictsDeferredMessage extends AmeMessage {
  const AmeConflictsDeferredMessage(this.count);
  final int count;
}

final class AmeFirstImportRequiredFolderMessage extends AmeMessage {
  const AmeFirstImportRequiredFolderMessage(this.name);
  final String name;
}

final class AmeIncompleteChangesMessage extends AmeMessage {
  const AmeIncompleteChangesMessage(this.count);
  final String count;
}

final class AmeMovedToAlbumMessage extends AmeMessage {
  const AmeMovedToAlbumMessage(this.name);
  final String name;
}

final class AmePendingChangesMessage extends AmeMessage {
  const AmePendingChangesMessage(this.count);
  final String count;
}

final class AmeRecordsClearedMessage extends AmeMessage {
  const AmeRecordsClearedMessage(this.count);
  final int count;
}

final class AmeRemovedFolderMessage extends AmeMessage {
  const AmeRemovedFolderMessage(this.name);
  final String name;
}

final class AmeRemovedFoldersMessage extends AmeMessage {
  const AmeRemovedFoldersMessage(this.count);
  final int count;
}

final class AmeRetryingChangesMessage extends AmeMessage {
  const AmeRetryingChangesMessage(this.count);
  final String count;
}

final class AmeSynchronizationBlockedFolderMessage extends AmeMessage {
  const AmeSynchronizationBlockedFolderMessage(this.name);
  final String name;
}

final class AmeSynchronizationUncheckedFolderMessage extends AmeMessage {
  const AmeSynchronizationUncheckedFolderMessage(this.name);
  final String name;
}
