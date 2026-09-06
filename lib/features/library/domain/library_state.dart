import "library_models.dart";

enum LibraryScanPhase { discovering, finalizing }

enum LibraryTaskKind { import, update, remove }

enum LibraryStatus {
  empty,
  choosingDirectory,
  scanning,
  pausing,
  cancelling,
  removing,
  refreshing,
  completed,
  cancelled,
  paused,
  stale,
  failed,
}

class LibraryState {
  const LibraryState({
    this.status = LibraryStatus.empty,
    this.scanId,
    this.rootPath,
    this.displayRootPath,
    this.taskKind,
    this.removingRootId,
    this.removingRootDisplayPath,
    this.isRemovalCommitted = false,
    this.completedRemovalRootId,
    this.rootRemovalCompletionSequence = 0,
    this.roots = const [],
    this.assets = const [],
    this.recentIssues = const [],
    this.visitedEntries = 0,
    this.stagedAssetCount = 0,
    this.scanPhase = LibraryScanPhase.discovering,
    this.validatedAssetCount = 0,
    this.validationAssetCount = 0,
    this.issueCount = 0,
    this.itemLimit,
    this.entryLimit,
    this.catalogPath,
    this.catalogRevision,
    this.query = const LibraryGalleryQuery(),
    this.queryId = "",
    this.windowStartItemOffset = 0,
    this.previousCursor,
    this.nextCursor,
    this.timeline,
    this.activeTimeAnchor,
    this.queryAnchorResolution,
    this.isScanLimited = false,
    this.isResumingScan = false,
    this.isLoadingPage = false,
    this.isLoadingPreviousPage = false,
    this.isLoadingTimeline = false,
    this.isLoadingTimeAnchor = false,
    this.isLoadingVisibleRange = false,
    this.pageErrorMessage,
    this.previousPageErrorMessage,
    this.timeNavigationErrorMessage,
    this.errorMessage,
  });

  static const Object _unchanged = Object();

  final LibraryStatus status;
  final String? scanId;
  final String? rootPath;
  final String? displayRootPath;
  final LibraryTaskKind? taskKind;
  final String? removingRootId;
  final String? removingRootDisplayPath;
  final bool isRemovalCommitted;
  final String? completedRemovalRootId;
  final int rootRemovalCompletionSequence;
  final List<LibraryRoot> roots;
  final List<LibraryAsset> assets;
  final List<LibraryIssue> recentIssues;
  final int visitedEntries;
  final int stagedAssetCount;
  final LibraryScanPhase scanPhase;
  final int validatedAssetCount;
  final int validationAssetCount;
  final int issueCount;
  final int? itemLimit;
  final int? entryLimit;
  final String? catalogPath;
  final BigInt? catalogRevision;
  final LibraryGalleryQuery query;
  final String queryId;
  final int windowStartItemOffset;
  final LibraryCatalogCursor? previousCursor;
  final LibraryCatalogCursor? nextCursor;
  final LibraryTimeline? timeline;
  final LibraryTimeAnchor? activeTimeAnchor;
  final LibraryQueryAnchorResolution? queryAnchorResolution;
  final bool isScanLimited;
  final bool isResumingScan;
  final bool isLoadingPage;
  final bool isLoadingPreviousPage;
  final bool isLoadingTimeline;
  final bool isLoadingTimeAnchor;
  final bool isLoadingVisibleRange;
  final String? pageErrorMessage;
  final String? previousPageErrorMessage;
  final String? timeNavigationErrorMessage;
  final String? errorMessage;

  bool get isScanning =>
      status == LibraryStatus.scanning ||
      status == LibraryStatus.pausing ||
      status == LibraryStatus.cancelling;

  bool get isProcessing =>
      status == LibraryStatus.choosingDirectory ||
      isScanning ||
      status == LibraryStatus.removing ||
      status == LibraryStatus.refreshing;

  bool get isCommittedRemovalReloadPending =>
      taskKind == LibraryTaskKind.remove && isRemovalCommitted;

  bool get isBusy =>
      isProcessing ||
      status == LibraryStatus.paused ||
      isLoadingTimeAnchor ||
      isCommittedRemovalReloadPending;

  bool get hasMoreAssets => nextCursor != null;

  bool get hasPreviousAssets => previousCursor != null;

  factory LibraryState.fromSnapshot(
    LibrarySnapshot snapshot, {
    LibraryGalleryQuery query = const LibraryGalleryQuery(),
  }) {
    final issueCount = snapshot.roots.fold(
      0,
      (total, root) => total + root.issueCount,
    );
    return LibraryState(
      status: snapshot.roots.isEmpty
          ? LibraryStatus.empty
          : LibraryStatus.completed,
      roots: snapshot.roots,
      assets: snapshot.assets,
      issueCount: issueCount,
      catalogPath: snapshot.catalogPath,
      catalogRevision: snapshot.revision,
      query: query,
      queryId: snapshot.queryId,
      previousCursor: snapshot.previousCursor,
      nextCursor: snapshot.nextCursor,
      queryAnchorResolution: snapshot.queryAnchorResolution,
    );
  }

  LibraryState copyWith({
    LibraryStatus? status,
    Object? scanId = _unchanged,
    Object? rootPath = _unchanged,
    Object? displayRootPath = _unchanged,
    Object? taskKind = _unchanged,
    Object? removingRootId = _unchanged,
    Object? removingRootDisplayPath = _unchanged,
    bool? isRemovalCommitted,
    Object? completedRemovalRootId = _unchanged,
    int? rootRemovalCompletionSequence,
    List<LibraryRoot>? roots,
    List<LibraryAsset>? assets,
    List<LibraryIssue>? recentIssues,
    int? visitedEntries,
    int? stagedAssetCount,
    LibraryScanPhase? scanPhase,
    int? validatedAssetCount,
    int? validationAssetCount,
    int? issueCount,
    Object? itemLimit = _unchanged,
    Object? entryLimit = _unchanged,
    Object? catalogPath = _unchanged,
    Object? catalogRevision = _unchanged,
    LibraryGalleryQuery? query,
    String? queryId,
    int? windowStartItemOffset,
    Object? previousCursor = _unchanged,
    Object? nextCursor = _unchanged,
    Object? timeline = _unchanged,
    Object? activeTimeAnchor = _unchanged,
    Object? queryAnchorResolution = _unchanged,
    bool? isScanLimited,
    bool? isResumingScan,
    bool? isLoadingPage,
    bool? isLoadingPreviousPage,
    bool? isLoadingTimeline,
    bool? isLoadingTimeAnchor,
    bool? isLoadingVisibleRange,
    Object? pageErrorMessage = _unchanged,
    Object? previousPageErrorMessage = _unchanged,
    Object? timeNavigationErrorMessage = _unchanged,
    Object? errorMessage = _unchanged,
  }) {
    return LibraryState(
      status: status ?? this.status,
      scanId: scanId == _unchanged ? this.scanId : scanId as String?,
      rootPath: rootPath == _unchanged ? this.rootPath : rootPath as String?,
      displayRootPath: displayRootPath == _unchanged
          ? this.displayRootPath
          : displayRootPath as String?,
      taskKind: taskKind == _unchanged
          ? this.taskKind
          : taskKind as LibraryTaskKind?,
      removingRootId: removingRootId == _unchanged
          ? this.removingRootId
          : removingRootId as String?,
      removingRootDisplayPath: removingRootDisplayPath == _unchanged
          ? this.removingRootDisplayPath
          : removingRootDisplayPath as String?,
      isRemovalCommitted: isRemovalCommitted ?? this.isRemovalCommitted,
      completedRemovalRootId: completedRemovalRootId == _unchanged
          ? this.completedRemovalRootId
          : completedRemovalRootId as String?,
      rootRemovalCompletionSequence:
          rootRemovalCompletionSequence ?? this.rootRemovalCompletionSequence,
      roots: roots ?? this.roots,
      assets: assets ?? this.assets,
      recentIssues: recentIssues ?? this.recentIssues,
      visitedEntries: visitedEntries ?? this.visitedEntries,
      stagedAssetCount: stagedAssetCount ?? this.stagedAssetCount,
      scanPhase: scanPhase ?? this.scanPhase,
      validatedAssetCount: validatedAssetCount ?? this.validatedAssetCount,
      validationAssetCount: validationAssetCount ?? this.validationAssetCount,
      issueCount: issueCount ?? this.issueCount,
      itemLimit: itemLimit == _unchanged ? this.itemLimit : itemLimit as int?,
      entryLimit: entryLimit == _unchanged
          ? this.entryLimit
          : entryLimit as int?,
      catalogPath: catalogPath == _unchanged
          ? this.catalogPath
          : catalogPath as String?,
      catalogRevision: catalogRevision == _unchanged
          ? this.catalogRevision
          : catalogRevision as BigInt?,
      query: query ?? this.query,
      queryId: queryId ?? this.queryId,
      windowStartItemOffset:
          windowStartItemOffset ?? this.windowStartItemOffset,
      previousCursor: previousCursor == _unchanged
          ? this.previousCursor
          : previousCursor as LibraryCatalogCursor?,
      nextCursor: nextCursor == _unchanged
          ? this.nextCursor
          : nextCursor as LibraryCatalogCursor?,
      timeline: timeline == _unchanged
          ? this.timeline
          : timeline as LibraryTimeline?,
      activeTimeAnchor: activeTimeAnchor == _unchanged
          ? this.activeTimeAnchor
          : activeTimeAnchor as LibraryTimeAnchor?,
      queryAnchorResolution: queryAnchorResolution == _unchanged
          ? this.queryAnchorResolution
          : queryAnchorResolution as LibraryQueryAnchorResolution?,
      isScanLimited: isScanLimited ?? this.isScanLimited,
      isResumingScan: isResumingScan ?? this.isResumingScan,
      isLoadingPage: isLoadingPage ?? this.isLoadingPage,
      isLoadingPreviousPage:
          isLoadingPreviousPage ?? this.isLoadingPreviousPage,
      isLoadingTimeline: isLoadingTimeline ?? this.isLoadingTimeline,
      isLoadingTimeAnchor: isLoadingTimeAnchor ?? this.isLoadingTimeAnchor,
      isLoadingVisibleRange:
          isLoadingVisibleRange ?? this.isLoadingVisibleRange,
      pageErrorMessage: pageErrorMessage == _unchanged
          ? this.pageErrorMessage
          : pageErrorMessage as String?,
      previousPageErrorMessage: previousPageErrorMessage == _unchanged
          ? this.previousPageErrorMessage
          : previousPageErrorMessage as String?,
      timeNavigationErrorMessage: timeNavigationErrorMessage == _unchanged
          ? this.timeNavigationErrorMessage
          : timeNavigationErrorMessage as String?,
      errorMessage: errorMessage == _unchanged
          ? this.errorMessage
          : errorMessage as String?,
    );
  }
}
