import "library_models.dart";

enum LibraryStatus {
  empty,
  choosingDirectory,
  scanning,
  pausing,
  cancelling,
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
    this.roots = const [],
    this.assets = const [],
    this.recentIssues = const [],
    this.visitedEntries = 0,
    this.stagedAssetCount = 0,
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
    this.isScanLimited = false,
    this.isResumingScan = false,
    this.isLoadingPage = false,
    this.isLoadingPreviousPage = false,
    this.isLoadingTimeline = false,
    this.isLoadingTimeAnchor = false,
    this.pageErrorMessage,
    this.previousPageErrorMessage,
    this.timeNavigationErrorMessage,
    this.errorMessage,
  });

  static const Object _unchanged = Object();

  final LibraryStatus status;
  final String? scanId;
  final String? rootPath;
  final List<LibraryRoot> roots;
  final List<LibraryAsset> assets;
  final List<LibraryIssue> recentIssues;
  final int visitedEntries;
  final int stagedAssetCount;
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
  final bool isScanLimited;
  final bool isResumingScan;
  final bool isLoadingPage;
  final bool isLoadingPreviousPage;
  final bool isLoadingTimeline;
  final bool isLoadingTimeAnchor;
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
      status == LibraryStatus.refreshing;

  bool get isBusy =>
      isProcessing || status == LibraryStatus.paused || isLoadingTimeAnchor;

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
    );
  }

  LibraryState copyWith({
    LibraryStatus? status,
    Object? scanId = _unchanged,
    Object? rootPath = _unchanged,
    List<LibraryRoot>? roots,
    List<LibraryAsset>? assets,
    List<LibraryIssue>? recentIssues,
    int? visitedEntries,
    int? stagedAssetCount,
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
    bool? isScanLimited,
    bool? isResumingScan,
    bool? isLoadingPage,
    bool? isLoadingPreviousPage,
    bool? isLoadingTimeline,
    bool? isLoadingTimeAnchor,
    Object? pageErrorMessage = _unchanged,
    Object? previousPageErrorMessage = _unchanged,
    Object? timeNavigationErrorMessage = _unchanged,
    Object? errorMessage = _unchanged,
  }) {
    return LibraryState(
      status: status ?? this.status,
      scanId: scanId == _unchanged ? this.scanId : scanId as String?,
      rootPath: rootPath == _unchanged ? this.rootPath : rootPath as String?,
      roots: roots ?? this.roots,
      assets: assets ?? this.assets,
      recentIssues: recentIssues ?? this.recentIssues,
      visitedEntries: visitedEntries ?? this.visitedEntries,
      stagedAssetCount: stagedAssetCount ?? this.stagedAssetCount,
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
      isScanLimited: isScanLimited ?? this.isScanLimited,
      isResumingScan: isResumingScan ?? this.isResumingScan,
      isLoadingPage: isLoadingPage ?? this.isLoadingPage,
      isLoadingPreviousPage:
          isLoadingPreviousPage ?? this.isLoadingPreviousPage,
      isLoadingTimeline: isLoadingTimeline ?? this.isLoadingTimeline,
      isLoadingTimeAnchor: isLoadingTimeAnchor ?? this.isLoadingTimeAnchor,
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
