import "../domain/library_models.dart";
import "../domain/library_state.dart";

const _recentIssueLimit = 20;

class LibraryScanTransition {
  const LibraryScanTransition({
    required this.state,
    this.shouldReloadCatalog = false,
  });

  final LibraryState state;
  final bool shouldReloadCatalog;
}

class LibraryScanSession {
  String? _activeScanId;
  RecoverableLibraryScan? _activeScan;
  RecoverableLibraryScan? _pausedScan;

  String? get activeScanId => _activeScanId;

  RecoverableLibraryScan? get pausedScan => _pausedScan;

  void begin(RecoverableLibraryScan scan) {
    _activeScanId = scan.scanId;
    _activeScan = scan;
    _pausedScan = null;
  }

  void restorePaused(RecoverableLibraryScan scan) {
    _activeScanId = null;
    _activeScan = null;
    _pausedScan = scan;
  }

  LibraryScanTransition apply(LibraryState state, LibraryScanUpdate update) {
    switch (update) {
      case LibraryScanStarted(
        :final scanId,
        :final rootPath,
        :final itemLimit,
        :final entryLimit,
      ):
        _activeScanId = scanId;
        return LibraryScanTransition(
          state: state.copyWith(
            status: LibraryStatus.scanning,
            scanId: scanId,
            displayRootPath: rootPath,
            itemLimit: itemLimit,
            entryLimit: entryLimit,
            scanPhase: LibraryScanPhase.discovering,
            validatedAssetCount: 0,
            validationAssetCount: 0,
          ),
        );
      case LibraryScanProgress(
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ):
        _updateActiveProgress(
          visitedEntries: visitedEntries,
          acceptedItems: acceptedItems,
          issueCount: issueCount,
        );
        return LibraryScanTransition(
          state: state.copyWith(
            visitedEntries: visitedEntries,
            stagedAssetCount: acceptedItems,
            scanPhase: LibraryScanPhase.discovering,
            validatedAssetCount: 0,
            validationAssetCount: 0,
            issueCount: issueCount,
          ),
        );
      case LibraryScanFinalizing(
        :final validatedItems,
        :final totalItems,
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ):
        _updateActiveProgress(
          visitedEntries: visitedEntries,
          acceptedItems: acceptedItems,
          issueCount: issueCount,
        );
        return LibraryScanTransition(
          state: state.copyWith(
            scanPhase: LibraryScanPhase.finalizing,
            validatedAssetCount: validatedItems,
            validationAssetCount: totalItems,
            visitedEntries: visitedEntries,
            stagedAssetCount: acceptedItems,
            issueCount: issueCount,
          ),
        );
      case LibraryAssetDiscovered():
        return LibraryScanTransition(
          state: state.copyWith(
            stagedAssetCount: state.stagedAssetCount + 1,
            visitedEntries: state.visitedEntries + 1,
          ),
        );
      case LibraryIssueDiscovered(:final issue):
        final issues = [...state.recentIssues, issue];
        return LibraryScanTransition(
          state: state.copyWith(
            issueCount: state.issueCount + 1,
            recentIssues: List.unmodifiable(
              issues.length > _recentIssueLimit
                  ? issues.sublist(issues.length - _recentIssueLimit)
                  : issues,
            ),
          ),
        );
      case LibraryScanCompleted(
        :final assetCount,
        :final issueCount,
        :final catalogPath,
        :final wasLimited,
      ):
        _clearActive();
        return LibraryScanTransition(
          state: state.copyWith(
            status: LibraryStatus.refreshing,
            stagedAssetCount: assetCount,
            issueCount: issueCount,
            catalogPath: catalogPath,
            isScanLimited: wasLimited,
            isResumingScan: false,
          ),
          shouldReloadCatalog: true,
        );
      case LibraryScanCancelled(:final issueCount):
        _clearActive();
        return LibraryScanTransition(
          state: state.copyWith(
            status: LibraryStatus.cancelled,
            issueCount: issueCount,
            isResumingScan: false,
          ),
        );
      case LibraryScanPaused(
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ):
        final activeScan = _activeScan;
        _clearActive();
        if (activeScan != null) {
          _pausedScan = RecoverableLibraryScan(
            scanId: activeScan.scanId,
            rootPath: activeScan.rootPath,
            displayRootPath: activeScan.displayRootPath,
            itemLimit: activeScan.itemLimit,
            entryLimit: activeScan.entryLimit,
            previewEdge: activeScan.previewEdge,
            visitedEntries: visitedEntries,
            acceptedItems: acceptedItems,
            issueCount: issueCount,
          );
        }
        return LibraryScanTransition(
          state: state.copyWith(
            status: LibraryStatus.paused,
            visitedEntries: visitedEntries,
            stagedAssetCount: acceptedItems,
            issueCount: issueCount,
            isResumingScan: false,
          ),
        );
      case LibraryScanStale(:final issueCount):
        _clearActive();
        return LibraryScanTransition(
          state: state.copyWith(
            status: LibraryStatus.stale,
            issueCount: issueCount,
            isResumingScan: false,
          ),
        );
      case LibraryScanFailed(:final code, :final message):
        _clearActive();
        return LibraryScanTransition(
          state: state.copyWith(
            status: LibraryStatus.failed,
            isResumingScan: false,
            errorMessage: LibraryScanFailure(
              code: code,
              message: message,
            ).toString(),
          ),
        );
    }
  }

  LibraryState fail(LibraryState state, Object error) {
    _clearActive();
    return state.copyWith(
      status: LibraryStatus.failed,
      isResumingScan: false,
      errorMessage: error.toString(),
    );
  }

  LibraryState finish(LibraryState state) {
    if (state.status != LibraryStatus.scanning &&
        state.status != LibraryStatus.pausing &&
        state.status != LibraryStatus.cancelling) {
      return state;
    }
    _clearActive();
    return state.copyWith(
      status: LibraryStatus.failed,
      isResumingScan: false,
      errorMessage: "The scan ended without a completion event",
    );
  }

  void _updateActiveProgress({
    required int visitedEntries,
    required int acceptedItems,
    required int issueCount,
  }) {
    final activeScan = _activeScan;
    if (activeScan == null) {
      return;
    }
    _activeScan = RecoverableLibraryScan(
      scanId: activeScan.scanId,
      rootPath: activeScan.rootPath,
      displayRootPath: activeScan.displayRootPath,
      itemLimit: activeScan.itemLimit,
      entryLimit: activeScan.entryLimit,
      previewEdge: activeScan.previewEdge,
      visitedEntries: visitedEntries,
      acceptedItems: acceptedItems,
      issueCount: issueCount,
    );
  }

  void _clearActive() {
    _activeScanId = null;
    _activeScan = null;
  }
}
