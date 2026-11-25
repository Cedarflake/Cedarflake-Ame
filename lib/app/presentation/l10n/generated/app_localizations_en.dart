// ignore_for_file: deprecated_member_use

// ignore: unused_import
import 'package:intl/intl.dart' as intl;
import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get updateSettings => 'Update settings';

  @override
  String get updatePolicy => 'Automatic updates';

  @override
  String get updateChannel => 'Update channel';

  @override
  String get updateManual => 'Check manually';

  @override
  String get updateCheckAutomatically => 'Check automatically';

  @override
  String get updateDownloadAutomatically => 'Download automatically';

  @override
  String get updateStable => 'Stable';

  @override
  String get updatePreview => 'Preview';

  @override
  String get updateCheck => 'Check for updates';

  @override
  String get updateView => 'View update';

  @override
  String get updateRestart => 'Restart and update';

  @override
  String get updateTitle => 'Update Ame';

  @override
  String get updateAvailable => 'An update is available';

  @override
  String get updateCurrent => 'Ame is up to date';

  @override
  String get updateChecking => 'Checking for updates';

  @override
  String get updateDownloading => 'Downloading update';

  @override
  String get updateVerifying => 'Verifying update';

  @override
  String get updateStopping => 'Stopping download';

  @override
  String get updateReady => 'Update ready';

  @override
  String get updateRestarting => 'Restarting Ame';

  @override
  String get updateDownload => 'Download update';

  @override
  String get updateLater => 'Later';

  @override
  String get updateStop => 'Stop download';

  @override
  String get updateReadyHelp => 'Restart Ame to install the update.';

  @override
  String get updateUnavailable =>
      'Use an installed or portable release to check for updates.';

  @override
  String get updateNetworkFailure =>
      'Could not check for or download updates. Try again later.';

  @override
  String get updateReleaseFailure =>
      'Update information is incomplete. Try again later.';

  @override
  String get updatePackageFailure =>
      'The update package is incomplete. Download it again.';

  @override
  String get updateStorageFailure =>
      'Could not save the update. Check available space and folder permissions.';

  @override
  String get updateInstallationFailure =>
      'The update could not be installed. You can keep using the current version.';

  @override
  String get updateShutdownFailure =>
      'An operation could not finish. Try again later.';

  @override
  String get updateInstalled => 'Ame has been updated';

  @override
  String get updateCancelled => 'Update installation cancelled';

  @override
  String get updateFailed => 'Update failed';

  @override
  String get settingsTitle => 'Settings';

  @override
  String get settingsPersonalization => 'Personalization';

  @override
  String get settingsLanguage => 'App language';

  @override
  String get settingsLanguageHelp => 'Choose the language used in Ame.';

  @override
  String get languageSystem => 'Follow system';

  @override
  String get languageChinese => '简体中文';

  @override
  String get languageEnglish => 'English';

  @override
  String get languageJapanese => '日本語';

  @override
  String get settingsTheme => 'App theme';

  @override
  String get settingsThemeHelp =>
      'Choose a light or dark appearance. The accent color follows Windows.';

  @override
  String get themeLight => 'Light';

  @override
  String get themeDark => 'Dark';

  @override
  String get settingsBrowsing => 'Browsing';

  @override
  String get settingsWheel => 'Mouse wheel';

  @override
  String get settingsWheelHelp =>
      'Choose what scrolling does when viewing a photo.';

  @override
  String get settingsWheelZoom => 'Zoom in or out';

  @override
  String get settingsWheelNavigate => 'Go to the previous or next photo';

  @override
  String get settingsOpenPhoto => 'Opening photos';

  @override
  String get settingsOpenPhotoHelp =>
      'Automatic fits large photos to the window and shows smaller photos at actual size.';

  @override
  String get viewerAutomatic => 'Automatic';

  @override
  String get viewerFitWindow => 'Fit to window';

  @override
  String get viewerActualSize => 'Actual size';

  @override
  String get settingsThumbnailSpeed => 'Thumbnail loading speed';

  @override
  String get settingsThumbnailSpeedHelp =>
      'Higher speeds load more thumbnails at once and use more resources.';

  @override
  String get sizeSmall => 'Small';

  @override
  String get sizeMedium => 'Medium';

  @override
  String get sizeLarge => 'Large';

  @override
  String get settingsAbout => 'About';

  @override
  String get settingsVersion => 'Version';

  @override
  String get settingsLicenses => 'Open-source notices';

  @override
  String get settingsLicensesHelp =>
      'View the open-source software and licenses used by Ame.';

  @override
  String get settingsSaveFailed => 'Couldn\'t save settings';

  @override
  String get startupFailureTitle => 'Cedarflake Ame couldn\'t start';

  @override
  String get copyDiagnostics => 'Copy diagnostics';

  @override
  String get diagnosticsCopied => 'Diagnostics copied';

  @override
  String get startupUnsupportedCatalog =>
      'This library was created with an unsupported version. Copy the diagnostics for troubleshooting.';

  @override
  String get startupDatabaseError =>
      'Ame couldn\'t read the library. Check that the drive is available, then restart Ame. If it still won\'t open, copy the diagnostics.';

  @override
  String get startupCatalogError =>
      'Restart Ame to try opening the library again. If it still won\'t open, copy the diagnostics.';

  @override
  String get startupComponentError =>
      'An app component couldn\'t load. Restart Ame. If it still won\'t start, copy the diagnostics.';

  @override
  String get choose => 'Choose';

  @override
  String get notSet => 'Not set';

  @override
  String get notifications => 'Notifications';

  @override
  String get notificationsEmpty => 'No notifications';

  @override
  String get notificationHistory => 'History';

  @override
  String get acknowledge => 'Got it';

  @override
  String get notificationDetails => 'Notification details';

  @override
  String get sourceDirectory => 'Folder';

  @override
  String get technicalCode => 'Technical code';

  @override
  String get close => 'Close';

  @override
  String get operationFailedDetails =>
      'The operation didn\'t finish. Details are available.';

  @override
  String get retry => 'Retry';

  @override
  String get errorCode => 'Error code';

  @override
  String get openLocation => 'Open location';

  @override
  String get openLocationFailed => 'Couldn\'t open this location. Try again.';

  @override
  String get loading => 'Loading';

  @override
  String elapsedTime(String duration) {
    return 'Running for $duration';
  }

  @override
  String get occurredAt => 'Time';

  @override
  String elapsedSeconds(int seconds) {
    return '$seconds s';
  }

  @override
  String elapsedMinutes(int minutes, int seconds) {
    return '$minutes min $seconds s';
  }

  @override
  String elapsedHours(int hours, int minutes) {
    return '$hours h $minutes min';
  }

  @override
  String get notificationsUnread => 'Notifications, unread messages';

  @override
  String get notificationsRead => 'Notifications, no unread messages';

  @override
  String get windowMinimize => 'Minimize';

  @override
  String get windowRestore => 'Restore';

  @override
  String get windowMaximize => 'Maximize';

  @override
  String get chooseFolder => 'Choose folder';

  @override
  String get albumCreated => 'Album created';

  @override
  String get albumRenamed => 'Album renamed';

  @override
  String get albumIconChanged => 'Album icon changed';

  @override
  String get albumDeleted => 'Album deleted';

  @override
  String get removedFromAlbum => 'Removed from album';

  @override
  String movedToAlbum(String name) {
    return 'Moved to “$name”';
  }

  @override
  String addedToAlbum(String name) {
    return 'Added to “$name”';
  }

  @override
  String get archiveExported => 'Organization records exported';

  @override
  String get backupSaved => 'Backup saved to the selected location.';

  @override
  String get archiveRestored => 'Organization records restored';

  @override
  String get unlinkedRecordsKept => 'Unlinked records were kept.';

  @override
  String get favoriteAdded => 'Added to favorites';

  @override
  String get favoriteRemoved => 'Removed from favorites';

  @override
  String get unlinkedRecordsCleared => 'Unlinked records cleared';

  @override
  String recordsCleared(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Cleared $countString records.',
      one: 'Cleared $countString record.',
    );
    return '$_temp0 Your photos and albums were kept.';
  }

  @override
  String get changesSaveFailed => 'Couldn\'t save changes';

  @override
  String get organizationRecordChanged =>
      'The photo or its organization record changed. The current record was kept.';

  @override
  String get organizationHistoryChanged =>
      'The action history changed. Reopen the menu.';

  @override
  String get organizationSelectionChanged =>
      'The photo or its organization details changed. Try again.';

  @override
  String get organizationSelectionExpired =>
      'The photo or its organization details changed. Select the photos again.';

  @override
  String get changesUnconfirmed =>
      'Couldn\'t confirm that the changes were saved. Try again.';

  @override
  String get undoCompleted => 'Undone';

  @override
  String get redoCompleted => 'Redone';

  @override
  String get undoFailed => 'Couldn\'t undo';

  @override
  String get redoFailed => 'Couldn\'t redo';

  @override
  String get undo => 'Undo';

  @override
  String get changesSaved => 'Changes saved';

  @override
  String get libraryRefreshPending =>
      'The library hasn\'t refreshed yet. Try again.';

  @override
  String get locationRefreshFailed =>
      'Couldn\'t refresh library locations. Try again.';

  @override
  String get displayRefreshSuperseded =>
      'The library was saved. Another operation interrupted the display update. Refresh the view again.';

  @override
  String get deferredOrganization => 'Moved to Organize later';

  @override
  String get ignored => 'Ignored';

  @override
  String get restoredToInbox => 'Returned to To organize';

  @override
  String get relocationUnsupported =>
      'The current scan service doesn\'t support relocating this library.';

  @override
  String get albumRemovalRemaining =>
      'Album details changed. Retry the remaining items.';

  @override
  String albumRemovalFailed(String name) {
    return 'Couldn\'t delete “$name”. Try again.';
  }

  @override
  String get operationStopped => 'The operation stopped.';

  @override
  String get operationIncomplete =>
      'Couldn\'t finish the operation. Try again.';

  @override
  String get folderStillUpdating =>
      'The folder changed or is still updating. Try again later.';

  @override
  String get folderRemovedRefreshPending =>
      'The folder was removed. Close this window, then refresh the library.';

  @override
  String get folderRemovalFailed => 'Couldn\'t remove the folder. Try again.';

  @override
  String get scanEndedWithoutResult =>
      'The library update ended without a result.';

  @override
  String get updatePausedRestart =>
      'The library update paused. Start the update again.';

  @override
  String get sourceChangedDuringUpdate =>
      'Source files changed during the update. Try again.';

  @override
  String get updateSourceChanged => 'The sources changed. Select them again.';

  @override
  String get addToAlbum => 'Add to album';

  @override
  String get moveToAnotherAlbum => 'Move to another album';

  @override
  String get addOrMoveToAlbum => 'Add or move to album';

  @override
  String filterFormat(String format) {
    return 'Format: $format';
  }

  @override
  String filterWidth(String range) {
    return 'Width: $range px';
  }

  @override
  String filterHeight(String range) {
    return 'Height: $range px';
  }

  @override
  String filterCamera(String camera) {
    return 'Camera: $camera';
  }

  @override
  String filterFileSize(String range, String unit) {
    return 'File size: $range $unit';
  }

  @override
  String get organizationEnterFailed =>
      'Couldn\'t enter organization mode. Try again.';

  @override
  String get galleryReturnFailed =>
      'Couldn\'t return to the library. Try again.';

  @override
  String get organizationScopeFailed =>
      'Couldn\'t switch the organization scope. Try again.';

  @override
  String get conflictOpenFailed =>
      'Couldn\'t open the photos with conflicting albums. Try again.';

  @override
  String get organizationPositionUnsaved =>
      'Your place hasn\'t been saved. Try again.';

  @override
  String get organizationPositionUnreadable =>
      'Couldn\'t read your saved place. Try again.';

  @override
  String get organizationResumeFailed =>
      'Couldn\'t resume where you left off. Try again.';

  @override
  String get organizationNextFailed =>
      'Your changes were saved. Try opening the next photo again.';

  @override
  String get organizationLibraryUpdating =>
      'The library is updating. Try switching photos again.';

  @override
  String get organizationViewChanged =>
      'This view changed. Try switching photos again.';

  @override
  String get unknown => 'Unknown';

  @override
  String get smartAnalysisInterrupted => 'Smart album analysis was interrupted';

  @override
  String get smartResultsAvailable => 'You can still browse existing results.';

  @override
  String get albumConflict => 'Conflicting albums';

  @override
  String conflictsDeferred(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other:
          '$countString groups need albums. They were moved to Organize later.',
      one: '$countString group needs an album. It was moved to Organize later.',
    );
    return '$_temp0';
  }

  @override
  String get viewAction => 'View';

  @override
  String get duplicateWorkspace => 'Find duplicate photos';

  @override
  String get openInExplorer => 'Open in File Explorer';

  @override
  String get openInExplorerFailed => 'Couldn\'t open File Explorer';

  @override
  String get stopping => 'Stopping';

  @override
  String get duplicateRunning => 'Looking for duplicates';

  @override
  String get duplicateIncomplete => 'Duplicate search didn\'t finish';

  @override
  String get waitingForBackground => 'Waiting for background work';

  @override
  String get paused => 'Paused';

  @override
  String get duplicateEmptyAfter => 'No duplicates found in the photos checked';

  @override
  String get duplicateWaitResults => 'Duplicate photos will appear here.';

  @override
  String get duplicateLocationsChanged =>
      'Some photos in this group have changed location';

  @override
  String get resultsUnavailable => 'Couldn\'t load results';

  @override
  String get viewDuplicates => 'View duplicate photos';

  @override
  String get backToDuplicates => 'Back to duplicate photos';

  @override
  String get duplicateGroups => 'Duplicate groups';

  @override
  String get ignoredGroupsEmpty => 'No ignored groups';

  @override
  String get similarGroups => 'Similar groups';

  @override
  String get similarityLevel => 'Similarity';

  @override
  String get similarityStrict => 'Strict';

  @override
  String get similarityStandard => 'Standard';

  @override
  String get similarityBroad => 'Broad';

  @override
  String get viewSimilar => 'View similar photos';

  @override
  String get similarEmpty => 'No similar groups found in the photos checked';

  @override
  String get similarWaitResults => 'Similar photos will appear here';

  @override
  String get comparePhotos => 'Compare photos';

  @override
  String get chooseComparison => 'Choose photos to compare';

  @override
  String get syncComparison => 'Sync zoom and pan';

  @override
  String get resetComparison => 'Reset comparison view';

  @override
  String get zoomIn => 'Zoom in';

  @override
  String get zoomOut => 'Zoom out';

  @override
  String get setPreferred => 'Set as preferred';

  @override
  String get selectPhoto => 'Select photo';

  @override
  String get deselectPhoto => 'Deselect photo';

  @override
  String get clearPreferred => 'Clear preference';

  @override
  String get ignoreGroup => 'Ignore group';

  @override
  String get restoreGroup => 'Restore group';

  @override
  String get selectionSaveFailed => 'Couldn\'t save your selection. Try again.';

  @override
  String groupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString groups',
      one: '$countString group',
    );
    return '$_temp0';
  }

  @override
  String photoCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString photos',
      one: '$countString photo',
    );
    return '$_temp0';
  }

  @override
  String deselectNamedGroup(String name) {
    return 'Deselect photo group: $name';
  }

  @override
  String selectNamedGroup(String name) {
    return 'Select photo group: $name';
  }

  @override
  String get filter => 'Filter';

  @override
  String get showSubfolders => 'Show subfolders';

  @override
  String get hideSubfolders => 'Hide subfolders';

  @override
  String get allPhotosFilter => 'All photos';

  @override
  String locationCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString locations',
      one: '$countString location',
    );
    return '$_temp0';
  }

  @override
  String duplicatePhotoCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString duplicate photos',
      one: '$countString duplicate photo',
    );
    return '$_temp0';
  }

  @override
  String physicalFileCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString files',
      one: '$countString file',
    );
    return '$_temp0';
  }

  @override
  String get cloudOnly => 'Cloud only';

  @override
  String get folderOffline => 'Folder offline';

  @override
  String get temporarilyInaccessible => 'Temporarily unavailable';

  @override
  String get deleteChangedFile =>
      'The file changed and was kept. Review the latest photo, then select it again to confirm.';

  @override
  String get noPhotosToDelete => 'No photos to delete';

  @override
  String aboutToDeletePhotos(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'About to delete $countString photos',
      one: 'About to delete $countString photo',
    );
    return '$_temp0';
  }

  @override
  String confirmDeleteFiles(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Delete $countString files?',
      one: 'Delete $countString file?',
    );
    return '$_temp0';
  }

  @override
  String confirmDeleteWholeGroup(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Delete all $countString files in this group?',
      one: 'Delete the only file in this group?',
    );
    return '$_temp0';
  }

  @override
  String confirmDeleteGroups(int groups, int files) {
    final intl.NumberFormat groupsNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String groupsString = groupsNumberFormat.format(groups);
    final intl.NumberFormat filesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String filesString = filesNumberFormat.format(files);

    String _temp0 = intl.Intl.pluralLogic(
      groups,
      locale: localeName,
      other: '$groupsString photo groups',
      one: '$groupsString photo group',
    );
    String _temp1 = intl.Intl.pluralLogic(
      files,
      locale: localeName,
      other: '$filesString files',
      one: '$filesString file',
    );
    return 'Delete $_temp0 containing $_temp1?';
  }

  @override
  String get destinationFolderDirect => 'Directly in the destination folder';

  @override
  String get byCategory => 'By category';

  @override
  String get byAlbum => 'By album';

  @override
  String get byDate => 'By date';

  @override
  String get bySourceFolder => 'By source folder';

  @override
  String get byAlbumYear => 'By album and year';

  @override
  String get dateFolderRule =>
      'Create YYYY-MM folders using library dates. Files without dates are skipped.';

  @override
  String get albumYearRule =>
      'Use album and year folders based on library dates, for example Travel/2024.';

  @override
  String get unknownDestinationRule =>
      'Files without a destination are skipped. A report is saved in the destination folder.';

  @override
  String get copy => 'Copy';

  @override
  String get move => 'Move';

  @override
  String get rename => 'Rename';

  @override
  String get deleteAction => 'Delete';

  @override
  String filesCompleted(String action, int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString files',
      one: '$countString file',
    );
    return '$action: $_temp0';
  }

  @override
  String get deleted => 'Deleted';

  @override
  String get completed => 'Completed';

  @override
  String filesIncomplete(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'Incomplete: $countString';
  }

  @override
  String filesStopped(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'Stopped: $countString';
  }

  @override
  String filesSkipped(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'Skipped: $countString';
  }

  @override
  String get noFilesProcessed => 'No files were processed.';

  @override
  String get reportNotNeeded => 'No report needed';

  @override
  String get reportSaved => 'Report saved';

  @override
  String get reportFailed => 'Couldn\'t create the report';

  @override
  String get checkFileOperation => 'Check file operation';

  @override
  String get generateReport => 'Create report';

  @override
  String preparingOperation(String action) {
    return 'Preparing: $action';
  }

  @override
  String runningOperation(String action) {
    return 'In progress: $action';
  }

  @override
  String operationEnded(String action) {
    return 'Finished: $action';
  }

  @override
  String operationStoppedLabel(String action) {
    return 'Stopped: $action';
  }

  @override
  String get fileOperationNeedsAttention => 'File operation needs attention';

  @override
  String get notStarted => 'Not started';

  @override
  String get verifyResults => 'Results need checking';

  @override
  String get moveIncomplete => 'Move incomplete';

  @override
  String get filesProcessedRefreshPending =>
      'Files processed, library update pending';

  @override
  String get incomplete => 'Incomplete';

  @override
  String get stopped => 'Stopped';

  @override
  String get resultsToVerify => 'Results awaiting review';

  @override
  String get duplicateVerificationPending =>
      'These duplicate groups are still being checked. Try again later.';

  @override
  String get selectedPhotosChanged =>
      'The selected photos changed. Review the library and select them again.';

  @override
  String get selectedFilesChanged =>
      'The selected files changed. Review them and select them again.';

  @override
  String get deleteFilesUnverified =>
      'The files changed or couldn\'t be verified. Review the latest photos and select them again to confirm deletion.';

  @override
  String get filesUnverified =>
      'The files changed or couldn\'t be verified. Review the latest files and select them again.';

  @override
  String get multipleFileLocations =>
      'These photos have multiple file locations. Clear the selection, then use each photo\'s context menu.';

  @override
  String get destinationMatchesSource =>
      'This destination matches the original file. Choose another folder or name.';

  @override
  String get destinationUnavailable =>
      'Couldn\'t open the destination folder. Choose it again.';

  @override
  String get selectedFilesUnreadable =>
      'Couldn\'t read the selected files. Refresh the library and select them again.';

  @override
  String get retainedPhotoChanged =>
      'The photo to keep changed or couldn\'t be read. Select the photo to keep again. No files were deleted.';

  @override
  String get fileNameUnavailable =>
      'This file name isn\'t available. Choose another name.';

  @override
  String get fileOperationPrepareFailed =>
      'Couldn\'t prepare the file operation. Select the files and try again.';

  @override
  String get fileOperationBusy =>
      'Another file operation is running. Try again later.';

  @override
  String get fileHistoryCheckRequired =>
      'Check the result in file operation history before continuing.';

  @override
  String get fileOperationCancelled => 'This operation was canceled.';

  @override
  String get reportWriteFailed =>
      'The operation result was kept. You can create the report again from history.';

  @override
  String get fileProcessingUncertain =>
      'Some files may still need attention. Check file operation history.';

  @override
  String get moreFilters => 'More filters';

  @override
  String get filterPhotos => 'Filter photos';

  @override
  String get clearFilters => 'Clear filters';

  @override
  String get cancel => 'Cancel';

  @override
  String get apply => 'Apply';

  @override
  String get date => 'Date';

  @override
  String get dateField => 'Date type';

  @override
  String get startDate => 'Start date';

  @override
  String get endDate => 'End date';

  @override
  String get fileFormat => 'File format';

  @override
  String get fileSize => 'File size';

  @override
  String get width => 'Width';

  @override
  String get height => 'Height';

  @override
  String get minimum => 'Minimum';

  @override
  String get maximum => 'Maximum';

  @override
  String get unit => 'Unit';

  @override
  String get dimensions => 'Image dimensions';

  @override
  String get minimumWidth => 'Minimum width';

  @override
  String get maximumWidth => 'Maximum width';

  @override
  String get minimumHeight => 'Minimum height';

  @override
  String get maximumHeight => 'Maximum height';

  @override
  String get cameraInformation => 'Camera information';

  @override
  String get cameraHint => 'Make or model';

  @override
  String get invalidExtension =>
      'Choose up to 16 formats. Use up to 16 letters or digits per format.';

  @override
  String get invalidNumber => 'Enter zero or a positive number.';

  @override
  String get invalidInteger => 'Enter zero or a positive whole number.';

  @override
  String get fractionalBytes =>
      'The converted size must be a whole number of bytes.';

  @override
  String get numberTooLarge => 'This number is too large.';

  @override
  String get reversedRange => 'The maximum must be at least the minimum.';

  @override
  String get invalidDate => 'Enter a valid date, such as 2026-01-31.';

  @override
  String get reversedDates =>
      'The end date must be on or after the start date.';

  @override
  String get invalidCamera => 'Enter a make or model of up to 128 characters.';

  @override
  String get captureDate => 'Date taken';

  @override
  String get createdDate => 'Date created';

  @override
  String get modifiedDate => 'Date modified';

  @override
  String get favorites => 'Favorites';

  @override
  String get albums => 'Albums';

  @override
  String get smartAlbums => 'Smart albums';

  @override
  String get modelDownloading => 'Downloading the smart album model';

  @override
  String get modelVerifying => 'Checking the model';

  @override
  String get modelEnabling => 'Enabling smart albums';

  @override
  String get modelEnabled => 'Smart albums enabled';

  @override
  String get modelEnableCancelled => 'Smart album setup canceled';

  @override
  String get modelEnableFailed => 'Couldn\'t enable smart albums';

  @override
  String get modelPreparing => 'Preparing the smart album model';

  @override
  String get modelConfigUnreadable =>
      'Couldn\'t read the model settings. Try again.';

  @override
  String get modelDownloadRetained =>
      'Your download progress was saved. You can try again.';

  @override
  String get modelEnableRetry =>
      'Try enabling smart albums again. Already downloaded model files will be reused.';

  @override
  String get albumNameInvalid =>
      'Enter an album name between 1 and 128 characters.';

  @override
  String get albumNameTaken =>
      'An album already has this name. Choose another name.';

  @override
  String get albumMissing =>
      'This album no longer exists. Choose another album.';

  @override
  String get organizationDataChanged =>
      'The photo or album details changed. Review them and try again.';

  @override
  String get organizationPositionChanged =>
      'Your place changed. Reopen this photo.';

  @override
  String get organizationReselectPhotos =>
      'The photo or its organization details changed. Cancel and select the photos again.';

  @override
  String get noActionablePhotos =>
      'No photos are available for this action. Select the photos again.';

  @override
  String get previousOperationRunning =>
      'The previous operation is still running. Try again later.';

  @override
  String get chooseConflictAlbum =>
      'This photo has conflicting albums. Choose one album.';

  @override
  String get photoInformationUpdating =>
      'Photo details are updating. Try again later.';

  @override
  String get saveResultUnknown => 'Couldn\'t confirm the save. Try again.';

  @override
  String get stoppingCurrentTask => 'Stopping the current task';

  @override
  String get resumeOrCancelPaused => 'Resume or cancel the paused task first';

  @override
  String get loadingLibrary => 'Loading library…';

  @override
  String get selectingFolder => 'Choosing folder…';

  @override
  String checkingFolderPhotos(String name) {
    return 'Checking photos in “$name”…';
  }

  @override
  String updatingFolder(String name) {
    return 'Updating “$name”…';
  }

  @override
  String addingFolder(String name) {
    return 'Adding “$name”…';
  }

  @override
  String get pausingEllipsis => 'Pausing…';

  @override
  String get stoppingEllipsis => 'Stopping…';

  @override
  String get loadingPhotos => 'Loading photos…';

  @override
  String get updateLibraryCancelled => 'Library update canceled';

  @override
  String get addFolderCancelled => 'Adding folder canceled';

  @override
  String get updateLibraryPaused => 'Library update paused';

  @override
  String get addFolderPaused => 'Adding folder paused';

  @override
  String get filesChangedRetry => 'Files changed. Try again.';

  @override
  String get updateLibraryFailed => 'Couldn\'t update the library';

  @override
  String get addFolderFailed => 'Couldn\'t add folder';

  @override
  String get updateComplete => 'Update complete';

  @override
  String get addComplete => 'Folder added';

  @override
  String totalPhotos(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString photos in total',
      one: '$countString photo in total',
    );
    return '$_temp0';
  }

  @override
  String photosAdded(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Added $countString photos',
      one: 'Added $countString photo',
    );
    return '$_temp0';
  }

  @override
  String photosValidated(int checked, int total, int files) {
    final intl.NumberFormat checkedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String checkedString = checkedNumberFormat.format(checked);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);
    final intl.NumberFormat filesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String filesString = filesNumberFormat.format(files);

    return 'Photos checked: $checkedString/$totalString · Files checked: $filesString';
  }

  @override
  String filesCheckedPhotosFound(int files, int photos) {
    final intl.NumberFormat filesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String filesString = filesNumberFormat.format(files);
    final intl.NumberFormat photosNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String photosString = photosNumberFormat.format(photos);

    return 'Files checked: $filesString · Photos found: $photosString';
  }

  @override
  String get chooseFolderAgain => 'Choose folder again';

  @override
  String get retryRemoval => 'Retry removal';

  @override
  String get photos => 'Photos';

  @override
  String get removeFromLater => 'Remove from Organize later';

  @override
  String get organizeLater => 'Organize later';

  @override
  String get unignore => 'Stop ignoring';

  @override
  String get ignoreSelected => 'Ignore selected photos';

  @override
  String get ignorePhoto => 'Ignore photo';

  @override
  String get photoAnalysisPaused => 'Photo analysis paused';

  @override
  String get searchUnavailable => 'Search is temporarily unavailable';

  @override
  String get retryAnalysis => 'Retry analysis';

  @override
  String get searchContent => 'Image content';

  @override
  String get searchNamePath => 'File name and path';

  @override
  String get searchCurrentScope => 'Current view';

  @override
  String get searchEntireLibrary => 'Entire library';

  @override
  String get findRelatedPhotos => 'Find related photos';

  @override
  String get updateSearchResults => 'Update search results';

  @override
  String get retrySearch => 'Retry search';

  @override
  String get contentSearchHint => 'Describe the photos you want to find';

  @override
  String get fileSearchHint => 'Search file names or paths';

  @override
  String get contentSearchEmptyHint =>
      'Try another description, adjust the filters, or search the entire library.';

  @override
  String get filterSearchEmptyHint => 'Adjust or clear the filters.';

  @override
  String get searchNeedsModel =>
      'Install the photo analysis model in Settings first.';

  @override
  String get searchDescriptionTooLong =>
      'Enter a description of up to 512 characters.';

  @override
  String get searchPartialAnalysis =>
      'Some photos haven\'t been analyzed. Available results were kept. Retry analysis.';

  @override
  String get searchLibraryChanged =>
      'The library just changed. Run the search again.';

  @override
  String get searchPhotoChanged =>
      'This photo changed or couldn\'t be read. Select it again.';

  @override
  String get searchAnalysisUnavailable =>
      'Photo analysis is unavailable. Try searching again.';

  @override
  String get searchIndexDamaged =>
      'Search data is damaged. Try again to rebuild it.';

  @override
  String get searchStorageUnavailable =>
      'Couldn\'t access search data. Try again.';

  @override
  String get searchFailedRetained =>
      'Search is unavailable. The previous photo list was kept. Try again.';

  @override
  String searchCoverage(int indexed, int total, String status) {
    final intl.NumberFormat indexedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String indexedString = indexedNumberFormat.format(indexed);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return 'Sorted by relevance · Photos analyzed: $indexedString/$totalString$status';
  }

  @override
  String get analysisStorageFullSuffix => ' · Analysis storage is full';

  @override
  String get partialResults => 'Partial results';

  @override
  String get showFilmstrip => 'Show thumbnails';

  @override
  String get hideFilmstrip => 'Hide thumbnails';

  @override
  String get scrollFilmstripLeft => 'Scroll left';

  @override
  String get scrollFilmstripRight => 'Scroll right';

  @override
  String get fitImageToWindow => 'Fit to window (0 / Ctrl+0)';

  @override
  String get showImageActualSize => 'Actual size (1 / Ctrl+1)';

  @override
  String get enterViewerFullscreen => 'Full screen (F11)';

  @override
  String get exitViewerFullscreen => 'Exit full screen (Esc / F11)';

  @override
  String get clearSearch => 'Clear search';

  @override
  String get library => 'Library';

  @override
  String get allImages => 'All photos';

  @override
  String get addFolder => 'Add folder to library';

  @override
  String get noFolder => 'No folders added yet';

  @override
  String get expandFolder => 'Expand folder';

  @override
  String get collapseFolder => 'Collapse folder';

  @override
  String get loadingFolders => 'Loading folders…';

  @override
  String get showMoreFolders => 'Show more folders';

  @override
  String get retryFolders => 'Reload folder';

  @override
  String get synchronized => 'Up to date';

  @override
  String get synchronizing => 'Updating';

  @override
  String get backgroundVerification => 'Checking';

  @override
  String get needsReconciliation => 'Update blocked';

  @override
  String get synchronizationUnconfirmed => 'Not checked yet';

  @override
  String get synchronizationRetryPending => 'Waiting to retry';

  @override
  String get continuityLiveOnly => 'Automatic updates unavailable';

  @override
  String get continuityUnavailable => 'Can\'t update automatically';

  @override
  String get sourceUnavailable => 'Unavailable';

  @override
  String get sourceAvailable => 'Available';

  @override
  String get sourceMissing => 'Folder missing';

  @override
  String get sourceOffline => 'Offline';

  @override
  String get sourceUnknown => 'Status unknown';

  @override
  String get addingSource => 'Adding';

  @override
  String get refreshingSourceDisplay => 'Refreshing';

  @override
  String get sourceAdded => 'Added';

  @override
  String get sourceDisplayReloadPending => 'Reload needed';

  @override
  String get sourceScanResultReadFailed => 'Couldn\'t read results';

  @override
  String get sourceReadingScanResult => 'Reading results';

  @override
  String get layout => 'Layout';

  @override
  String get sort => 'Sort';

  @override
  String get fileName => 'Name';

  @override
  String get newestAddedFirst => 'Recently added first';

  @override
  String get ascending => 'Ascending';

  @override
  String get descending => 'Descending';

  @override
  String get equalHeight => 'Equal height';

  @override
  String get square => 'Square';

  @override
  String get sizeMediumLong => 'Medium';

  @override
  String get more => 'More';

  @override
  String get selectAll => 'Select all';

  @override
  String get unknownCaptureDate => 'Date taken unknown';

  @override
  String get open => 'Open';

  @override
  String get viewInformation => 'View information';

  @override
  String get copyPath => 'Copy path';

  @override
  String get updateLibrary => 'Update library';

  @override
  String get relocateLibrary => 'Relocate folder';

  @override
  String get removeFromAme => 'Remove from Ame';

  @override
  String get updatingLibrary => 'Updating library…';

  @override
  String get emptyLibraryTitle => 'Build your photo library';

  @override
  String get emptyLibraryBody =>
      'Choose folders to browse their photos together in Ame.';

  @override
  String get importPhotos => 'Import';

  @override
  String get firstImportIncomplete => 'Folder not fully added';

  @override
  String get firstImportInProgress => 'Adding photos';

  @override
  String get firstImportInProgressDetail =>
      'Photos will appear here when the folder has been added.';

  @override
  String get pausingSource => 'Pausing';

  @override
  String get firstImportPaused => 'Adding photos paused';

  @override
  String get firstImportPausedDetail =>
      'Your progress was saved. You can continue later.';

  @override
  String get cancellingSource => 'Canceling';

  @override
  String get firstImportCancelled => 'Adding photos canceled';

  @override
  String get firstImportRetryDetail =>
      'Couldn\'t add this folder to the library.';

  @override
  String get firstImportGalleryDetail =>
      'You can add this folder again when you need it.';

  @override
  String get scanConfirmationPending => 'Couldn\'t read scan results';

  @override
  String get scanConfirmationDetail =>
      'The scanned photos can\'t be displayed yet.';

  @override
  String get readingScanResult => 'Reading scan results…';

  @override
  String get rereadScanResult => 'Read results again';

  @override
  String get galleryDisplayReloadPending => 'Couldn\'t load photos';

  @override
  String get galleryDisplayReloadDetail =>
      'The photos were added, but can\'t be displayed yet.';

  @override
  String get addFolderAwaitingScanResult =>
      'Couldn\'t read scan results. Wait before adding another folder.';

  @override
  String get addFolderAwaitingGalleryReload =>
      'Couldn\'t load the library. Wait before adding another folder.';

  @override
  String get noSearchResults => 'No matching photos';

  @override
  String get noSearchResultsHint => 'Try another name or path.';

  @override
  String get noSourceResults => 'No photos to display in this folder';

  @override
  String get noSourceResultsHint =>
      'Choose another folder or return to the library.';

  @override
  String get retryLoading => 'Reload';

  @override
  String get retryPreview => 'Reload preview';

  @override
  String get retryingPreview => 'Loading preview…';

  @override
  String get previewFormatUnrecognized => 'Unrecognized image format';

  @override
  String get previewUpdateRequired =>
      'Update the library, then reload the preview.';

  @override
  String get backToLibrary => 'Back to library';

  @override
  String get originalImageUnavailable => 'Couldn\'t open the original photo';

  @override
  String get originalImagePreviewFallback =>
      'Couldn\'t open the original. Showing the preview.';

  @override
  String get retryOriginalImage => 'Open again';

  @override
  String get viewerSourceChanged =>
      'This photo changed. Reload it to see the latest version.';

  @override
  String get viewerSourceMissing => 'This photo is no longer in the library.';

  @override
  String removingFromAme(String name) {
    return 'Removing “$name” from Ame…';
  }

  @override
  String get removingFolderDetail =>
      'The original folder and photos will be kept.';

  @override
  String get removeFolderFailed => 'Couldn\'t remove folder';

  @override
  String refreshingAfterRemoval(String name) {
    return 'Removed “$name”. Reloading…';
  }

  @override
  String get refreshingAfterRemovalDetail =>
      'The original folder and photos were kept.';

  @override
  String get removedFolderRefreshFailed =>
      'Folder removed, but photos couldn\'t be loaded';

  @override
  String get scanOverlapsStorage =>
      'This folder overlaps Ame\'s data storage. Choose another folder.';

  @override
  String get scanBusy =>
      'Another folder is updating. Wait for it to finish, then continue.';

  @override
  String get scanCancelFailed => 'Couldn\'t cancel. Try again.';

  @override
  String get parentScanIncomplete =>
      'The parent folder wasn\'t fully scanned. Check that you can open it and that the scan limit wasn\'t reached. Previously added subfolders were kept.';

  @override
  String get childScanPending =>
      'A subfolder scan is unfinished. Resume or cancel it before adding the parent folder.';

  @override
  String get childUpdateRequired =>
      'Update the existing subfolder before adding its parent.';

  @override
  String get childMergeFailed =>
      'Couldn\'t combine the existing subfolders. Check their locations and try again.';

  @override
  String get scanFolderFailed =>
      'Couldn\'t update this folder. Check that you can open it.';

  @override
  String get synchronizationRefreshFailureMessage =>
      'Reload to see the latest photos.';

  @override
  String get manualUpdateNotice => 'Some folders need a manual update';

  @override
  String get manualUpdateDetail =>
      'After reopening Ame, select Update library in the folder menu to include changes made while Ame was closed.';

  @override
  String synchronizationBlockedFolder(String name) {
    return 'Updates to “$name” are blocked';
  }

  @override
  String synchronizationUncheckedFolder(String name) {
    return '“$name” hasn\'t been checked yet';
  }

  @override
  String firstImportRequiredFolder(String name) {
    return '“$name” hasn\'t been fully added';
  }

  @override
  String get synchronizationSourceUnavailable => 'Couldn\'t open this folder.';

  @override
  String get synchronizationSourceUnhealthy =>
      'Automatic updates were interrupted.';

  @override
  String get synchronizationSourceStarting => 'Preparing automatic updates.';

  @override
  String get synchronizationEvidenceGap =>
      'Some file changes haven\'t reached the library yet.';

  @override
  String get synchronizationCapacityExceeded =>
      'Many files changed. The library update hasn\'t finished yet.';

  @override
  String get synchronizationNeedsReconciliation =>
      'The library update hasn\'t finished yet.';

  @override
  String get synchronizationCompleted =>
      'The library now includes the folder changes.';

  @override
  String get synchronizationAutomaticRetry =>
      'Some updates are pending. Ame will retry automatically.';

  @override
  String get synchronizationRecoveryBlocked =>
      'Choose Update library in the folder menu to try again.';

  @override
  String get synchronizationLegacyRecoveryAuthorityMissing =>
      'Choose Update library in the folder menu to resume automatic updates.';

  @override
  String get synchronizationMonitoringAccessDenied =>
      'Ame can\'t read changes in this folder. Check its access permissions.';

  @override
  String get synchronizationMonitoringEventIncomplete =>
      'Couldn\'t read some file changes.';

  @override
  String get synchronizationRecoveryFailed =>
      'The folder check didn\'t finish.';

  @override
  String get synchronizationPersistenceFailed =>
      'Couldn\'t save the library update.';

  @override
  String get synchronizationPreparing => 'Preparing update';

  @override
  String get synchronizationCheckingFolder => 'Checking folder';

  @override
  String get synchronizationCheckingChanges => 'Checking changes';

  @override
  String get synchronizationUpdatingLibrary => 'Updating library';

  @override
  String get synchronizationRetryLater => 'Will retry automatically';

  @override
  String get synchronizationScanningFolder => 'Scanning folder';

  @override
  String get synchronizationUpdateIncomplete => 'Update incomplete';

  @override
  String get synchronizationComplete => 'Sync complete';

  @override
  String get synchronizationWaitingFolder => 'Waiting for folder';

  @override
  String get libraryUpdateBlocked => 'Library update blocked';

  @override
  String pendingChanges(String count) {
    return 'Changes waiting: $count';
  }

  @override
  String retryingChanges(String count) {
    return 'Changes waiting to retry: $count';
  }

  @override
  String incompleteChanges(String count) {
    return 'Unfinished updates: $count';
  }

  @override
  String get changesToCheck => 'File changes still need checking';

  @override
  String get waitingForUpdate => 'Waiting for update';

  @override
  String get checkingPhotos => 'Checking photos';

  @override
  String get loadingPhotosShort => 'Loading photos';

  @override
  String get cancelled => 'Canceled';

  @override
  String get filesChanged => 'Files changed';

  @override
  String get scanCompleted => 'Scan complete.';

  @override
  String reloadRoot(String name) {
    return 'Reload $name';
  }

  @override
  String updateRoot(String name) {
    return 'Update library for $name';
  }

  @override
  String retryRoot(String name) {
    return 'Retry updating $name';
  }

  @override
  String progressPhotos(String phase, int checked, int total, int percent) {
    final intl.NumberFormat checkedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String checkedString = checkedNumberFormat.format(checked);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '$phase, photos: $checkedString of $totalString, $percent%';
  }

  @override
  String progressFiles(String phase, int files, int photos) {
    final intl.NumberFormat filesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String filesString = filesNumberFormat.format(files);
    final intl.NumberFormat photosNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String photosString = photosNumberFormat.format(photos);

    return '$phase, files checked: $filesString, photos found: $photosString';
  }

  @override
  String get duplicates => 'Duplicates';

  @override
  String get collapseSidebar => 'Collapse sidebar';

  @override
  String get expandSidebar => 'Expand sidebar';

  @override
  String get saveSidebarFailed => 'Couldn\'t save sidebar settings';

  @override
  String get changePhotoFailed => 'Couldn\'t switch photos';

  @override
  String removedFolders(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Removed $countString folders from Ame',
      one: 'Removed $countString folder from Ame',
    );
    return '$_temp0';
  }

  @override
  String get saveGallerySettingsFailed => 'Couldn\'t save gallery settings';

  @override
  String get filePathCopied => 'File path copied';

  @override
  String get copyPathFailed => 'Couldn\'t copy the path';

  @override
  String get confirmRemoveFolder => 'Remove folder from Ame?';

  @override
  String confirmRemoveFolderDetail(String name) {
    return 'Photos in “$name” will no longer appear in Ame. The original folder and its photos will be kept.';
  }

  @override
  String removedFolder(String name) {
    return 'Removed “$name” from Ame';
  }

  @override
  String get seekDateFailed => 'Couldn\'t go to the selected date';

  @override
  String get scrollUp => 'Scroll up';

  @override
  String scrollPosition(int percent) {
    return 'Scroll position: $percent%';
  }

  @override
  String get scrollDown => 'Scroll down';

  @override
  String get createAlbum => 'New album';

  @override
  String get renameAlbum => 'Rename album';

  @override
  String changeAlbumIcon(String name) {
    return 'Change icon for $name';
  }

  @override
  String get confirmDeleteAlbum => 'Delete album?';

  @override
  String get save => 'Save';

  @override
  String get saving => 'Saving';

  @override
  String get deleteAlbumImpact =>
      'Photo groups in this album will move to Ignored. Photo files and favorites will be kept.';

  @override
  String get albumName => 'Album name';

  @override
  String get chooseAlbumIcon => 'Choose album icon';

  @override
  String get albumIconTravel => 'Travel';

  @override
  String get albumIconNature => 'Nature';

  @override
  String get albumIconPets => 'Pets';

  @override
  String get albumIconPeople => 'People';

  @override
  String get albumIconFood => 'Food';

  @override
  String get albumIconArt => 'Art';

  @override
  String get albumIconWork => 'Work';

  @override
  String get albumIconStudy => 'Study';

  @override
  String get albumIconGames => 'Games';

  @override
  String get albumIconMusic => 'Music';

  @override
  String get albumIconMemories => 'Memories';

  @override
  String confirmDeleteAlbums(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Delete $countString albums?',
      one: 'Delete $countString album?',
    );
    return '$_temp0';
  }

  @override
  String get deleteAlbumsImpact =>
      'Photo groups in these albums will move to Ignored. Photo files and favorites will be kept.';

  @override
  String get deleteAlbum => 'Delete album';

  @override
  String get selectPageAlbums => 'Select all albums on this page';

  @override
  String get selectAllAlbums => 'Select all albums';

  @override
  String get deleteSelectedAlbums => 'Delete selected albums';

  @override
  String get albumsEmpty => 'No albums yet';

  @override
  String get previousPage => 'Previous page';

  @override
  String get nextPage => 'Next page';

  @override
  String get albumsUnavailable => 'Couldn\'t load albums';

  @override
  String get changeIcon => 'Change icon';

  @override
  String get chooseAlbum => 'Choose album';

  @override
  String get updatingAlbums => 'Updating albums';

  @override
  String get createFirstAlbum => 'Create an album first.';

  @override
  String get loadingAlbums => 'Loading albums';

  @override
  String get albumsLoadFailed => 'Couldn\'t load albums.';

  @override
  String get removeFromAlbum => 'Remove from album';

  @override
  String get albumGroupConflict =>
      'This group has conflicting albums. Choose an album.';

  @override
  String get albumDetailsUnavailable =>
      'Couldn\'t read album details. Select to retry.';

  @override
  String get albumsSidebarHint =>
      'Choose Favorites or an album in the sidebar, or create an album.';

  @override
  String get file => 'File';

  @override
  String get path => 'Path';

  @override
  String get fileType => 'Type';

  @override
  String get imageDimensions => 'Dimensions';

  @override
  String get size => 'Size';

  @override
  String get cameraMake => 'Camera make';

  @override
  String get cameraModel => 'Camera model';

  @override
  String get timeTaken => 'Time taken';

  @override
  String get timeSource => 'Time source';

  @override
  String get timeCreated => 'Time created';

  @override
  String get timeModified => 'Time modified';

  @override
  String get cameraOriginalTime => 'Original camera timestamp';

  @override
  String get digitizedTime => 'Digitized time';

  @override
  String get photoRecordedTime => 'Recorded image time';

  @override
  String get loadingAlbumAssignments => 'Loading album assignments';

  @override
  String get conflictAlbumsUnavailable =>
      'Couldn\'t read the albums in this conflict.';

  @override
  String get albumAssignmentsUpdated => 'Album assignments updated.';

  @override
  String get conflictAlbums => 'Albums in conflict';

  @override
  String deletedAlbumName(String name) {
    return '$name (deleted)';
  }

  @override
  String get similarKind => 'Similar';

  @override
  String get duplicateKind => 'Duplicate';

  @override
  String deletePhotoKind(String kind) {
    return 'Delete photos ($kind)';
  }

  @override
  String get saveSelectionFailed => 'Couldn\'t save selection';

  @override
  String get tryAgain => 'Try again.';

  @override
  String similarPhotoCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString similar photos',
      one: '$countString similar photo',
    );
    return '$_temp0';
  }

  @override
  String get updatingFileLocations => 'Updating file locations';

  @override
  String get deleteSimilarPhotos => 'Delete similar photos';

  @override
  String get deleteDuplicatePhotos => 'Delete duplicate photos';

  @override
  String get deleteSelected => 'Delete selected';

  @override
  String get loadingFileLocations => 'Loading file locations';

  @override
  String get checkingPhotoContent => 'Checking photo content';

  @override
  String get favoritesUnavailable => 'Couldn\'t read favorite status';

  @override
  String get savingChanges => 'Saving changes';

  @override
  String get loadingFavoriteStatus => 'Loading favorite status';

  @override
  String get removeFavorite => 'Remove from favorites';

  @override
  String get addFavorite => 'Add to favorites';

  @override
  String get readingPhotos => 'Reading photos';

  @override
  String photoOperation(String action) {
    return '$action photos';
  }

  @override
  String fileCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString files',
      one: '$countString file',
    );
    return '$_temp0';
  }

  @override
  String get destinationFolder => 'Destination folder';

  @override
  String get change => 'Change';

  @override
  String get folderStructure => 'Folder structure';

  @override
  String get fileNameLabel => 'File name';

  @override
  String get permanentDeleteWarning =>
      'These files will be permanently deleted. They can\'t be recovered from the Recycle Bin.';

  @override
  String get recycleWarning =>
      'These files will be moved to the Windows Recycle Bin.';

  @override
  String get deletePermanently => 'Delete permanently';

  @override
  String get back => 'Back';

  @override
  String get operationReport => 'Operation report';

  @override
  String get affectedFiles => 'Affected files';

  @override
  String selectedCountShort(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString selected';
  }

  @override
  String get reportFile => 'Report file';

  @override
  String get reportHistoryUnavailable =>
      'Couldn\'t read report history. Try again.';

  @override
  String get loadingReport => 'Loading report';

  @override
  String get noExportHistory => 'No exports yet';

  @override
  String get reportPending => 'Report hasn\'t been created';

  @override
  String get noReportNeeded => 'No exceptions to report';

  @override
  String get reportWriteFailedStatus =>
      'Couldn\'t write the report. File operation results were kept.';

  @override
  String get createReport => 'Create report';

  @override
  String get regenerate => 'Generate again';

  @override
  String get updatingReport => 'Updating report';

  @override
  String get latestRecords => 'Latest records';

  @override
  String get earlierRecords => 'Earlier records';

  @override
  String get stop => 'Stop';

  @override
  String get viewHistory => 'View history';

  @override
  String get noExceptionsReport =>
      'There were no exceptions. A report isn\'t needed.';

  @override
  String get loadingOperationHistory => 'Loading operation history';

  @override
  String verifiedFiles(int checked, int total) {
    final intl.NumberFormat checkedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String checkedString = checkedNumberFormat.format(checked);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return 'Files checked: $checkedString of $totalString';
  }

  @override
  String get reportSavedDestination =>
      'The report was saved in the destination folder.';

  @override
  String processedFiles(int processed, int total) {
    final intl.NumberFormat processedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String processedString = processedNumberFormat.format(processed);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return 'Files processed: $processedString of $totalString';
  }

  @override
  String get fileHistoryUnavailable =>
      'Couldn\'t read file operation history. Try again.';

  @override
  String get confirmContinueMove => 'Continue moving this file?';

  @override
  String get continueMove => 'Continue move';

  @override
  String get fileOperationHistory => 'File operation history';

  @override
  String get updatingFileRecords => 'Updating file records';

  @override
  String get fileRecords => 'File records';

  @override
  String get loadingFileRecords => 'Loading file records';

  @override
  String get noFileOperations => 'No file operations yet';

  @override
  String get fileOperationDetails => 'File operation details';

  @override
  String get checkResults => 'Check results';

  @override
  String get recycleBinOpenFailed =>
      'Couldn\'t open the Recycle Bin. Open it from the Windows desktop.';

  @override
  String get openRecycleBin => 'Open Recycle Bin';

  @override
  String get duplicatesOnly => 'Duplicates only';

  @override
  String get similarOnly => 'Similar photos only';

  @override
  String get mergeDuplicates => 'Group duplicates';

  @override
  String removeFilter(String label) {
    return 'Remove $label';
  }

  @override
  String selectedItems(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString items selected',
      one: '$countString item selected',
    );
    return '$_temp0';
  }

  @override
  String duplicateGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString duplicate groups',
      one: '$countString duplicate group',
    );
    return '$_temp0';
  }

  @override
  String similarGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString similar groups',
      one: '$countString similar group',
    );
    return '$_temp0';
  }

  @override
  String deduplicatedCount(String summary, int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString unique photos',
      one: '$countString unique photo',
    );
    return '$summary · $_temp0';
  }

  @override
  String organizationCounts(
    int total,
    int organized,
    int pending,
    int ignored,
  ) {
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);
    final intl.NumberFormat organizedNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String organizedString = organizedNumberFormat.format(organized);
    final intl.NumberFormat pendingNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String pendingString = pendingNumberFormat.format(pending);
    final intl.NumberFormat ignoredNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String ignoredString = ignoredNumberFormat.format(ignored);

    return 'Photos: $totalString · Organized: $organizedString · To organize: $pendingString · Ignored: $ignoredString';
  }

  @override
  String get organizationMode => 'Organization mode';

  @override
  String get exitOrganization => 'Exit organization mode';

  @override
  String get noPendingPhotos => 'No photos to organize';

  @override
  String get noDeferredPhotos => 'No photos to organize later';

  @override
  String get noIgnoredPhotos => 'No ignored photos';

  @override
  String get organizationEmptyHint =>
      'Switch queues or leave organization mode from the More menu.';

  @override
  String get noCategoryPhotos => 'No photos in this category yet';

  @override
  String get categoryEmptyHint =>
      'Choose another category or return to the library.';

  @override
  String get noAlbumPhotos => 'No photos in this album yet';

  @override
  String get albumEmptyHint =>
      'Open a photo in the library to add it to an album.';

  @override
  String get noFavoritePhotos => 'No favorites yet';

  @override
  String get favoritesEmptyHint => 'Open a photo to add it to favorites.';

  @override
  String get filmstripSettingFailed =>
      'Couldn\'t save thumbnail settings. Try again.';

  @override
  String photoPosition(int position, int total) {
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return '$position of $totalString';
  }

  @override
  String clearFilter(String label) {
    return 'Clear $label';
  }

  @override
  String get hide => 'Hide';

  @override
  String get downloadSmartModel => 'Download smart album model';

  @override
  String get smartModelDownloadDetail =>
      'Download about 504 MB to analyze photos on this device. Complete model files already on disk will be reused.';

  @override
  String get downloadAndEnable => 'Download and enable';

  @override
  String get folders => 'Folders';

  @override
  String get removeSelectedFolders => 'Remove selected folders from Ame';

  @override
  String get resizeSidebar => 'Resize sidebar';

  @override
  String pixelCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString pixels',
      one: '$countString pixel',
    );
    return '$_temp0';
  }

  @override
  String get undoAction => 'Undo action';

  @override
  String get redoAction => 'Redo action';

  @override
  String get historyLoadRetry =>
      'Couldn\'t load action history. Select to retry.';

  @override
  String get organizationOptions => 'Organization options';

  @override
  String get albumsAndFavoritesUnavailable =>
      'Couldn\'t read albums and favorites';

  @override
  String get reopenMenuRetry => 'Reopen the menu to try again.';

  @override
  String get copyToFolder => 'Copy to folder';

  @override
  String get moveToFolder => 'Move to folder';

  @override
  String get photoListFailed => 'Couldn\'t load the photo list';

  @override
  String get continueOrganizing => 'Continue organizing';

  @override
  String get toOrganize => 'To organize';

  @override
  String photosRelatedTo(String name) {
    return 'Photos related to $name';
  }

  @override
  String get selectedAlbumConflict =>
      'Some selected photos have conflicting albums. Choose one album.';

  @override
  String get actionFailed => 'Couldn\'t finish the action';

  @override
  String get favorite => 'Favorite';

  @override
  String get preparing => 'Preparing';

  @override
  String get processing => 'Processing';

  @override
  String itemsCompleted(int completed, int total) {
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String totalString = totalNumberFormat.format(total);

    return 'Items completed: $completed of $totalString.';
  }

  @override
  String get sortOrderSaveFailed =>
      'Couldn\'t save the new order. Drag the item again.';

  @override
  String confirmRemoveFolders(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: 'Remove $countString folders from Ame?',
      one: 'Remove $countString folder from Ame?',
    );
    return '$_temp0';
  }

  @override
  String get removeFoldersImpact =>
      'Photos in these folders will disappear from Ame. Album assignments, favorites and organization records unique to these folders will be removed. Copies in other sources will keep their records. Your original folders and files will be kept.';

  @override
  String selectedCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return '$countString selected';
  }

  @override
  String get selectedLong => 'Selected';

  @override
  String get selectedShort => 'Selected';

  @override
  String get deselectAll => 'Deselect all';

  @override
  String selectAllNamed(String label) {
    return 'Select all $label';
  }

  @override
  String collapseSection(String section) {
    return 'Collapse $section';
  }

  @override
  String expandSection(String section) {
    return 'Expand $section';
  }

  @override
  String deselectNamed(String name) {
    return 'Deselect $name';
  }

  @override
  String selectNamed(String name) {
    return 'Select $name';
  }

  @override
  String get noClassificationResults => 'No classification results yet';

  @override
  String get downloadModel => 'Download model';

  @override
  String get loadingSmartAlbums => 'Loading smart albums';

  @override
  String get categoriesUnavailable => 'Couldn\'t load categories';

  @override
  String recentlyAddedCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString photos',
      one: '$countString photo',
    );
    return 'Recently added · $_temp0';
  }

  @override
  String previouslyAddedCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString photos',
      one: '$countString photo',
    );
    return 'Added earlier · $_temp0';
  }

  @override
  String get recentlyAdded => 'Recently added';

  @override
  String get previouslyAdded => 'Earlier';

  @override
  String get pause => 'Pause';

  @override
  String get resume => 'Resume';

  @override
  String get libraryUpdates => 'Library updates';

  @override
  String get clearFinishedTasks => 'Clear finished tasks';

  @override
  String cancelRootUpdate(String name) {
    return 'Cancel update for $name';
  }

  @override
  String closeRootUpdate(String name) {
    return 'Dismiss update for $name';
  }

  @override
  String rootUpdateProgress(String name) {
    return 'Update progress for $name';
  }

  @override
  String categoryLabel(String name) {
    return 'Category: $name';
  }

  @override
  String get previousPhotoShortcut => 'Previous photo (←)';

  @override
  String get nextPhotoShortcut => 'Next photo (→)';

  @override
  String get photoCopied => 'Photo copied to clipboard';

  @override
  String get photoTooLargeToCopy =>
      'This photo is too large to copy to the clipboard';

  @override
  String get copyPhotoFailed => 'Couldn\'t copy the photo. Try again.';

  @override
  String get reloadThumbnails => 'Reload thumbnails';

  @override
  String get toggleFullscreenFailed =>
      'Couldn\'t switch full screen. Try again.';

  @override
  String get readingPhotoStatus => 'Reading photo status';

  @override
  String get rereadPhotoStatus => 'Read photo status again';

  @override
  String get copyPhoto => 'Copy photo';

  @override
  String get rotateClockwise => 'Rotate clockwise 90°';

  @override
  String get zoomOutShortcut => 'Zoom out (- / Ctrl+-)';

  @override
  String get zoomInShortcut => 'Zoom in (+ / Ctrl++)';

  @override
  String get archiveFileType => 'Ame organization records';

  @override
  String get exportAction => 'Export';

  @override
  String get chooseBackup => 'Choose backup';

  @override
  String get inspect => 'Check';

  @override
  String get restore => 'Restore';

  @override
  String get preparingBackup => 'Preparing backup';

  @override
  String get exportingArchive => 'Exporting organization records';

  @override
  String get checkingBackup => 'Checking backup';

  @override
  String get linkingPhotos => 'Matching photos in the library';

  @override
  String get restoringArchive => 'Restoring organization records';

  @override
  String archiveInProgress(String operation) {
    return 'Organization records: $operation in progress';
  }

  @override
  String get archiveOutsideLibrary =>
      'Choose a location outside the library and Ame data folders.';

  @override
  String get archiveFileExists =>
      'A file with this name already exists. Choose another name.';

  @override
  String get archiveVersionUnsupported =>
      'This backup version isn\'t supported. Choose a compatible backup.';

  @override
  String get archiveInvalid =>
      'This backup is incomplete or invalid. Choose another backup.';

  @override
  String get archiveLibraryChanged =>
      'The library or its records changed. Check the backup again before restoring.';

  @override
  String get archiveChanged =>
      'The selected backup changed. Check it again before restoring.';

  @override
  String get archiveAlreadyRunning =>
      'Another backup operation is running. Try again later.';

  @override
  String get archiveUnreadable =>
      'Couldn\'t read this backup. Check that the file is available on this device.';

  @override
  String get archiveIoFailed =>
      'Couldn\'t access the backup. Check the selected file and destination, then try again.';

  @override
  String get archiveConflictsWithCurrent =>
      'This backup conflicts with existing records. Your current records were kept. Choose another backup.';

  @override
  String get archiveResultUnknown => 'Couldn\'t confirm the result. Try again.';

  @override
  String get settingsSaveRetry => 'Couldn\'t save settings. Try again.';

  @override
  String get unreadable => 'Couldn\'t read';

  @override
  String get askBeforeAdding => 'Ask before adding to an album';

  @override
  String get askBeforeAddingHelp => 'Choose the destination album each time';

  @override
  String get defaultAlbum => 'Default album';

  @override
  String get defaultAlbumHelp =>
      'If no default is set, Ame asks before adding photos.';

  @override
  String get gpuSettingFailed => 'Couldn\'t update GPU acceleration settings';

  @override
  String get gpuSettingRetry =>
      'Try again. If photo analysis still fails, turn off GPU acceleration.';

  @override
  String get gpuAcceleration => 'GPU acceleration';

  @override
  String get loadingSettings => 'Loading settings';

  @override
  String get settingsReadFailed => 'Couldn\'t read settings. Try again.';

  @override
  String get switchingAnalysisDevice => 'Switching analysis device';

  @override
  String get smartAlbumsCpu =>
      'Smart albums are using the CPU to analyze photos';

  @override
  String smartAlbumsDevice(String name) {
    return 'Smart albums are analyzing photos with $name';
  }

  @override
  String get smartAlbumsPreferGpu =>
      'Smart albums use an available GPU for photo analysis';

  @override
  String get restoreOrganizationRecords => 'Restore organization records';

  @override
  String createdAt(String timestamp) {
    return 'Created: $timestamp';
  }

  @override
  String albumCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString albums',
      one: '$countString album',
    );
    return '$_temp0';
  }

  @override
  String photoGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString photo groups',
      one: '$countString photo group',
    );
    return '$_temp0';
  }

  @override
  String favoriteGroupCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString favorite groups',
      one: '$countString favorite group',
    );
    return '$_temp0';
  }

  @override
  String duplicateChoiceCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString duplicate choices',
      one: '$countString duplicate choice',
    );
    return '$_temp0';
  }

  @override
  String matchedGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString groups can be matched',
      one: '$countString group can be matched',
    );
    return '$_temp0';
  }

  @override
  String unmatchedGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString unmatched groups',
      one: '$countString unmatched group',
    );
    return '$_temp0';
  }

  @override
  String albumConflictCount(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString album conflicts',
      one: '$countString album conflict',
    );
    return '$_temp0';
  }

  @override
  String unresolvedCopies(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    String _temp0 = intl.Intl.pluralLogic(
      count,
      locale: localeName,
      other: '$countString preferred copies can\'t be matched yet',
      one: '$countString preferred copy can\'t be matched yet',
    );
    return '$_temp0';
  }

  @override
  String get includesOrganizationPosition =>
      'Includes your place in organization mode';

  @override
  String get restoreKeepsChoices =>
      'Your current choices and place will be kept. Original photos stay where they are.';

  @override
  String get unmatchedAwaitPhotos =>
      'Unmatched records will be kept until their photos can be matched.';

  @override
  String get filePickerFailed => 'Couldn\'t open the file picker';

  @override
  String get organizationRecords => 'Organization records';

  @override
  String get exportOrganizationRecords => 'Export organization records';

  @override
  String get exportArchiveHelp =>
      'Back up albums, favorites and organization progress. Original photos are excluded.';

  @override
  String get restoreArchiveHelp =>
      'Restore from a backup and keep your current choices';

  @override
  String get viewBackup => 'View backup';

  @override
  String get chooseFile => 'Choose file';

  @override
  String get noUnlinkedRecords => 'No matching unlinked records';

  @override
  String get unlinkedLessThanDay => 'Unlinked for less than a day';

  @override
  String unlinkedDays(int days) {
    String _temp0 = intl.Intl.pluralLogic(
      days,
      locale: localeName,
      other: 'Unlinked for $days days',
      one: 'Unlinked for $days day',
    );
    return '$_temp0';
  }

  @override
  String unlinkedRange(String shortest, int longest) {
    return 'Unlinked for $shortest–$longest days';
  }

  @override
  String get lessThanOne => 'less than 1';

  @override
  String get clearUnlinkedRecords => 'Clear unlinked records';

  @override
  String get checkingRecords => 'Checking records';

  @override
  String get cleanupScope => 'Records to clear';

  @override
  String get unlinkedNinetyDays => 'Unlinked for more than 90 days';

  @override
  String get unlinkedThirtyDays => 'Unlinked for more than 30 days';

  @override
  String get allUnlinkedRecords => 'All unlinked records';

  @override
  String get checkFailedRetry => 'Couldn\'t finish the check. Try again.';

  @override
  String get checkCancelled => 'Check canceled';

  @override
  String get clearRecords => 'Clear records';

  @override
  String cleanupCounts(int groups, int choices) {
    final intl.NumberFormat groupsNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String groupsString = groupsNumberFormat.format(groups);
    final intl.NumberFormat choicesNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String choicesString = choicesNumberFormat.format(choices);

    return 'Photo groups: $groupsString · Duplicate choices: $choicesString';
  }

  @override
  String cleanupAlbumGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'Groups with an album: $countString';
  }

  @override
  String cleanupFavoriteGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'Favorite groups: $countString';
  }

  @override
  String cleanupProgressGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'Groups with organization progress: $countString';
  }

  @override
  String cleanupClassificationGroups(int count) {
    final intl.NumberFormat countNumberFormat =
        intl.NumberFormat.decimalPattern(localeName);
    final String countString = countNumberFormat.format(count);

    return 'Groups with classification records: $countString';
  }

  @override
  String get cleanupRecordImpact =>
      'Albums and original photos will be kept. Importing the photos again won\'t restore these records.';

  @override
  String get cleanupUndoWarning =>
      'Clearing these records can\'t be undone. To restore them, use a backup you exported earlier.';

  @override
  String get cleanupRecordsHelp =>
      'Check albums, favorites and organization records that can\'t be matched to a photo';

  @override
  String get cleanUp => 'Clean up';

  @override
  String get stopNotConfirmed => 'The task hasn\'t stopped. Try again.';

  @override
  String get clearingRecords => 'Clearing records';

  @override
  String get checkAgain => 'Check again';

  @override
  String get recordsChanged =>
      'The records changed. Check them again before confirming.';

  @override
  String get cleanupResultUnknown =>
      'Couldn\'t confirm the cleanup result. Try again.';

  @override
  String get confirmClearThumbnails => 'Clear thumbnails?';

  @override
  String get confirmClearOldThumbnails => 'Clear the old thumbnail folder?';

  @override
  String get oldThumbnailFolder => 'Old thumbnail folder';

  @override
  String get clearThumbnailsImpact =>
      'This removes cached thumbnails. Your original photos are kept. Thumbnails may take a while to rebuild the next time you open the library.';

  @override
  String get clearOldThumbnailsImpact =>
      'Only thumbnails managed by Ame in this folder will be removed. Original photos and other files will be kept.';

  @override
  String get startCleanup => 'Start cleanup';

  @override
  String get cleanupAlreadyFinished => 'Cleanup has already finished.';

  @override
  String get storage => 'Storage';

  @override
  String get loadingStorageSettings => 'Loading storage settings';

  @override
  String get checkingStoragePaths =>
      'Checking the library data and thumbnail locations';

  @override
  String get storageSettingsUnavailable => 'Couldn\'t read storage settings';

  @override
  String get storageRestartRequired =>
      'Restart Ame to apply the new storage settings';

  @override
  String get existingFilesKept => 'Existing files stay in place';

  @override
  String get catalogStorageLocation => 'Library data location';

  @override
  String catalogStorageHelp(String path) {
    return 'Stores the library index and scan results\n$path';
  }

  @override
  String storageUsageWithDetail(String size, String detail) {
    return 'Using $size$detail';
  }

  @override
  String storageUsageDetail(String live, String reclaimable) {
    return ', about $live in use and $reclaimable reclaimable';
  }

  @override
  String get catalogLocationLocked => 'Can\'t change while a library exists';

  @override
  String get starting => 'Starting';

  @override
  String get cleanupNotNeeded => 'No cleanup needed';

  @override
  String get thumbnailLocation => 'Thumbnail location';

  @override
  String thumbnailStorageHelp(String path) {
    return 'Stores previews that can be rebuilt when needed\n$path';
  }

  @override
  String get retiredThumbnailDirectory => 'Previous thumbnail folder';

  @override
  String retiredThumbnailsHelp(String path) {
    return 'The new folder is in use. Confirm cleanup to remove Ame\'s thumbnails from the previous folder.\n$path';
  }

  @override
  String get cleaning => 'Cleaning up';

  @override
  String get cleanOldFolder => 'Clean up old folder';

  @override
  String get thumbnailBudget => 'Maximum thumbnail storage';

  @override
  String get thumbnailBudgetHelp =>
      'At the limit, unused thumbnails are removed automatically. Original photos are kept.';

  @override
  String storageUsage(String size) {
    return 'Using $size';
  }

  @override
  String get storageSaveFailed => 'Couldn\'t save storage settings';

  @override
  String get storageOverlapsSource =>
      'This storage location overlaps an added photo folder. Choose a separate folder for storage.';

  @override
  String get storageErrorDetails => 'View error details';

  @override
  String get storageHideErrorDetails => 'Hide error details';

  @override
  String get catalogMaintenanceStatusError =>
      'Couldn\'t read library maintenance status';

  @override
  String get thumbnailCleanupStatusError =>
      'Couldn\'t read thumbnail cleanup status';

  @override
  String get storageLocationPending =>
      'The current storage location stays in use until this finishes';

  @override
  String get optimizeCatalog => 'Optimize library storage';

  @override
  String get catalogWaiting => 'Library storage waiting for optimization';

  @override
  String get catalogInspecting => 'Checking library data';

  @override
  String get catalogWaitingIdle =>
      'Waiting for idle time to optimize library storage';

  @override
  String get catalogCheckingSpace => 'Checking space for optimization';

  @override
  String get catalogOptimizing => 'Optimizing the library database';

  @override
  String get catalogReclaiming => 'Reclaiming library storage';

  @override
  String get catalogOptimized => 'Library storage optimized';

  @override
  String get catalogOptimizationCancelled => 'Library optimization canceled';

  @override
  String get catalogOptimizationFailed => 'Library optimization failed';

  @override
  String reclaimableSpace(String size) {
    return 'About $size can be reclaimed';
  }

  @override
  String get catalogAutoReclaim =>
      'After a library is removed, Ame reclaims unused database space in the background. Original photos are kept.';

  @override
  String get catalogRemovalApplied =>
      'The library was removed. Background optimization will begin shortly.';

  @override
  String get catalogEstimating =>
      'Calculating storage in use and reclaimable space';

  @override
  String get catalogBusyWait =>
      'A library operation is in progress. Optimization will resume automatically.';

  @override
  String get catalogCheckingTempSpace =>
      'Checking the temporary disk space needed to rebuild the database';

  @override
  String catalogConverting(String size) {
    return 'Converting the database to reclaim about $size. A percentage isn\'t available during this step.';
  }

  @override
  String catalogReleasedRemaining(String released, String remaining) {
    return 'Reclaimed $released, about $remaining remaining';
  }

  @override
  String catalogReleased(String released) {
    return 'Reclaimed $released. Existing library data was kept.';
  }

  @override
  String get catalogStopped =>
      'Optimization stopped. The removal and existing data are preserved. You can try again later.';

  @override
  String get catalogRetryLater =>
      'Couldn\'t finish optimizing the library. Try again later.';

  @override
  String spaceRequired(String requiredSize, String available) {
    return 'Needs $requiredSize, $available available';
  }

  @override
  String get thumbnails => 'Thumbnails';

  @override
  String cleaningSubject(String subject) {
    return 'Cleaning up: $subject';
  }

  @override
  String cleanupSubjectComplete(String subject) {
    return 'Cleanup complete: $subject';
  }

  @override
  String cleanupSubjectCancelled(String subject) {
    return 'Cleanup canceled: $subject';
  }

  @override
  String cleanupSubjectFailed(String subject) {
    return 'Cleanup failed: $subject';
  }

  @override
  String get clearThumbnails => 'Clear thumbnails';

  @override
  String get thumbnailCleanupHelp =>
      'Thumbnails will be rebuilt when needed. Original photos are kept.';

  @override
  String get countingThumbnails => 'Counting thumbnails that can be removed';

  @override
  String cleanupFileProgress(String processed, String total, String size) {
    return 'Files processed: $processed of $total, $size reclaimed';
  }

  @override
  String cleanupFilesRemoved(String count, String size) {
    return 'Files removed: $count, space reclaimed: $size';
  }

  @override
  String cleanupBeforeStop(String count) {
    return 'Files removed before stopping: $count. Original photos were kept.';
  }

  @override
  String get thumbnailCleanupFailed => 'Couldn\'t finish clearing thumbnails';

  @override
  String cleanupFailedFiles(String count) {
    return 'Files that could not be removed: $count';
  }

  @override
  String get chooseThisFolder => 'Choose this folder';

  @override
  String get catalogCleanupAlreadyFinished =>
      'Library optimization has already finished.';

  @override
  String folderStatus(String name, String status) {
    return '“$name”: $status';
  }

  @override
  String galleryDay(DateTime date) {
    final intl.DateFormat dateDateFormat = intl.DateFormat.yMMMd(localeName);
    final String dateString = dateDateFormat.format(date);

    return '$dateString';
  }

  @override
  String galleryMonth(DateTime date) {
    final intl.DateFormat dateDateFormat = intl.DateFormat.yMMMM(localeName);
    final String dateString = dateDateFormat.format(date);

    return '$dateString';
  }

  @override
  String eventDateTime(DateTime date, DateTime time) {
    final intl.DateFormat dateDateFormat = intl.DateFormat.yMMMd(localeName);
    final String dateString = dateDateFormat.format(date);
    final intl.DateFormat timeDateFormat = intl.DateFormat.jm(localeName);
    final String timeString = timeDateFormat.format(time);

    return '$dateString $timeString';
  }

  @override
  String labelAndValue(String label, String value) {
    return '$label: $value';
  }

  @override
  String get inlineListSeparator => ', ';

  @override
  String get clauseSeparator => ', ';

  @override
  String get sentenceSeparator => '. ';

  @override
  String listSentence(String items) {
    return '$items.';
  }

  @override
  String get categoryIllustration => 'Illustration';

  @override
  String get categoryComic => 'Comics';

  @override
  String get categoryMeme => 'Memes';

  @override
  String get categoryScreenshot => 'Screenshots';

  @override
  String get categoryPhoto => 'Photography';

  @override
  String get categoryOther => 'Other';

  @override
  String modelDownloadProgress(int percent, double received, double total) {
    final intl.NumberFormat receivedNumberFormat =
        intl.NumberFormat.decimalPatternDigits(
          locale: localeName,
          decimalDigits: 1,
        );
    final String receivedString = receivedNumberFormat.format(received);
    final intl.NumberFormat totalNumberFormat =
        intl.NumberFormat.decimalPatternDigits(
          locale: localeName,
          decimalDigits: 1,
        );
    final String totalString = totalNumberFormat.format(total);

    return '$percent% · $receivedString / $totalString MB';
  }

  @override
  String photoTimestamp(DateTime date, DateTime time) {
    final intl.DateFormat dateDateFormat = intl.DateFormat.yMMMd(localeName);
    final String dateString = dateDateFormat.format(date);
    final intl.DateFormat timeDateFormat = intl.DateFormat.jms(localeName);
    final String timeString = timeDateFormat.format(time);

    return '$dateString $timeString';
  }
}
