import "library_models.dart";
import "library_primary_scan_snapshot.dart";
import "library_query_activity.dart";

export "library_primary_scan_snapshot.dart";
export "library_query_activity.dart";

/// Gallery data with a read-only projection of the independently owned scan task.
/// Flat task arguments seed initial snapshots; catalog publishers cannot update
/// an attached primary snapshot through these compatibility fields.
class LibraryState {
  const LibraryState({
    this._status = LibraryStatus.empty,
    this._scanId,
    this._rootPath,
    this._displayRootPath,
    this._taskKind,
    this.removingRootId,
    this.removingRootDisplayPath,
    this.isRemovalCommitted = false,
    this.completedRemovalRootId,
    this.rootRemovalCompletionSequence = 0,
    this.roots = const [],
    this.assets = const [],
    this._recentIssues = const [],
    this._visitedEntries = 0,
    this._stagedAssetCount = 0,
    this._scanPhase = LibraryScanPhase.discovering,
    this._validatedAssetCount = 0,
    this._validationAssetCount = 0,
    this._issueCount = 0,
    this._itemLimit,
    this._entryLimit,
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
    this._isScanLimited = false,
    this._isResumingScan = false,
    this.isLoadingPage = false,
    this.isLoadingPreviousPage = false,
    this.isLoadingTimeline = false,
    this.isLoadingTimeAnchor = false,
    this.isLoadingVisibleRange = false,
    this.queryActivity = const LibraryQueryIdle(),
    this.primaryScan,
    this.pageErrorMessage,
    this.previousPageErrorMessage,
    this.timeNavigationErrorMessage,
    this._errorMessage,
  });

  static const Object _unchanged = Object();

  final LibraryStatus _status;
  LibraryStatus get status => _taskKind == LibraryTaskKind.remove
      ? _status
      : isRefreshingQuery && !(primaryScan?.hasFeedback ?? false)
      ? LibraryStatus.refreshing
      : primaryScan != null &&
            !primaryScan!.hasFeedback &&
            (primaryScan!.status == LibraryStatus.empty ||
                primaryScan!.status == LibraryStatus.completed)
      ? (roots.isEmpty ? LibraryStatus.empty : LibraryStatus.completed)
      : primaryScan?.status ?? _status;
  final String? _scanId;
  String? get scanId => _usesPrimaryProjection ? primaryScan!.scanId : _scanId;
  final String? _rootPath;
  String? get rootPath =>
      _usesPrimaryProjection ? primaryScan!.rootPath : _rootPath;
  final String? _displayRootPath;
  String? get displayRootPath =>
      _usesPrimaryProjection ? primaryScan!.displayRootPath : _displayRootPath;
  final LibraryTaskKind? _taskKind;
  LibraryTaskKind? get taskKind => _taskKind == LibraryTaskKind.remove
      ? _taskKind
      : primaryScan == null
      ? _taskKind
      : primaryScan!.taskKind;
  final String? removingRootId;
  final String? removingRootDisplayPath;
  final bool isRemovalCommitted;
  final String? completedRemovalRootId;
  final int rootRemovalCompletionSequence;
  final List<LibraryRoot> roots;
  final List<LibraryAsset> assets;
  final List<LibraryIssue> _recentIssues;
  List<LibraryIssue> get recentIssues =>
      _usesPrimaryProjection ? primaryScan!.recentIssues : _recentIssues;
  final int _visitedEntries;
  int get visitedEntries =>
      _usesPrimaryProjection ? primaryScan!.visitedEntries : _visitedEntries;
  final int _stagedAssetCount;
  int get stagedAssetCount => _usesPrimaryProjection
      ? primaryScan!.stagedAssetCount
      : _stagedAssetCount;
  final LibraryScanPhase _scanPhase;
  LibraryScanPhase get scanPhase =>
      _usesPrimaryProjection ? primaryScan!.scanPhase : _scanPhase;
  final int _validatedAssetCount;
  int get validatedAssetCount => _usesPrimaryProjection
      ? primaryScan!.validatedAssetCount
      : _validatedAssetCount;
  final int _validationAssetCount;
  int get validationAssetCount => _usesPrimaryProjection
      ? primaryScan!.validationAssetCount
      : _validationAssetCount;
  final int _issueCount;
  int get issueCount =>
      _usesPrimaryProjection ? primaryScan!.issueCount : _issueCount;
  final int? _itemLimit;
  int? get itemLimit =>
      _usesPrimaryProjection ? primaryScan!.itemLimit : _itemLimit;
  final int? _entryLimit;
  int? get entryLimit =>
      _usesPrimaryProjection ? primaryScan!.entryLimit : _entryLimit;
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
  final bool _isScanLimited;
  bool get isScanLimited =>
      _usesPrimaryProjection ? primaryScan!.isScanLimited : _isScanLimited;
  final bool _isResumingScan;
  bool get isResumingScan =>
      _usesPrimaryProjection ? primaryScan!.isResumingScan : _isResumingScan;
  final bool isLoadingPage;
  final bool isLoadingPreviousPage;
  final bool isLoadingTimeline;
  final bool isLoadingTimeAnchor;
  final bool isLoadingVisibleRange;
  final LibraryQueryActivity queryActivity;
  bool get isRefreshingQuery => queryActivity is LibraryQueryLoading;
  final LibraryPrimaryScanSnapshot? primaryScan;

  bool get _usesPrimaryProjection =>
      primaryScan != null && _taskKind != LibraryTaskKind.remove;

  LibraryPrimaryScanSnapshot get primaryScanSnapshot =>
      primaryScan ??
      LibraryPrimaryScanSnapshot(
        status: _status,
        scanId: _scanId,
        rootPath: _rootPath,
        displayRootPath: _displayRootPath,
        taskKind: _taskKind,
        recentIssues: _recentIssues,
        visitedEntries: _visitedEntries,
        stagedAssetCount: _stagedAssetCount,
        scanPhase: _scanPhase,
        validatedAssetCount: _validatedAssetCount,
        validationAssetCount: _validationAssetCount,
        issueCount: _issueCount,
        itemLimit: _itemLimit,
        entryLimit: _entryLimit,
        isScanLimited: _isScanLimited,
        isResumingScan: _isResumingScan,
        errorMessage: _errorMessage,
      );

  bool get hasRetainedScan => primaryScanSnapshot.hasRetainedScan;

  String? get retainedScanRootId {
    if (!hasRetainedScan) {
      return null;
    }
    final task = primaryScanSnapshot;
    for (final root in roots) {
      if (root.path == task.rootPath ||
          root.displayPath == task.displayRootPath) {
        return root.id;
      }
    }
    return null;
  }

  final String? pageErrorMessage;
  final String? previousPageErrorMessage;
  final String? timeNavigationErrorMessage;
  final String? _errorMessage;
  String? get errorMessage =>
      _usesPrimaryProjection ? primaryScan!.errorMessage : _errorMessage;

  bool get isScanning =>
      status == LibraryStatus.scanning ||
      status == LibraryStatus.pausing ||
      status == LibraryStatus.cancelling;

  bool get isProcessing =>
      status == LibraryStatus.choosingDirectory ||
      isScanning ||
      status == LibraryStatus.removing ||
      status == LibraryStatus.refreshing ||
      isRefreshingQuery;

  bool get isTaskProcessing =>
      status == LibraryStatus.choosingDirectory ||
      isScanning ||
      status == LibraryStatus.removing ||
      status == LibraryStatus.refreshing ||
      status == LibraryStatus.discarding;

  bool get isCommittedRemovalReloadPending =>
      taskKind == LibraryTaskKind.remove && isRemovalCommitted;

  bool get isBusy =>
      isProcessing || isLoadingTimeAnchor || isCommittedRemovalReloadPending;

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
    LibraryQueryActivity? queryActivity,
    Object? primaryScan = _unchanged,
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
      queryActivity: queryActivity ?? this.queryActivity,
      primaryScan: primaryScan == _unchanged
          ? this.primaryScan
          : primaryScan as LibraryPrimaryScanSnapshot?,
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
