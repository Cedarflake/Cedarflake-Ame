import "library_models.dart";

enum LibraryScanPhase { discovering, finalizing }

enum LibraryTaskKind { import, update, remove }

enum LibraryScanPublication { unpublished, reloadPending, visible }

enum LibraryStatus {
  empty,
  choosingDirectory,
  scanning,
  pausing,
  cancelling,
  discarding,
  removing,
  refreshing,
  completed,
  cancelled,
  paused,
  stale,
  failed,
}

/// Task intent and progress, independent of any gallery query or root removal.
class LibraryPrimaryScanSnapshot {
  const LibraryPrimaryScanSnapshot({
    this.status = LibraryStatus.empty,
    this.scanId,
    this.rootPath,
    this.displayRootPath,
    this.taskKind,
    this.publication = LibraryScanPublication.unpublished,
    this.recentIssues = const [],
    this.visitedEntries = 0,
    this.stagedAssetCount = 0,
    this.scanPhase = LibraryScanPhase.discovering,
    this.validatedAssetCount = 0,
    this.validationAssetCount = 0,
    this.issueCount = 0,
    this.itemLimit,
    this.entryLimit,
    this.isScanLimited = false,
    this.isResumingScan = false,
    this.errorMessage,
  });

  final LibraryStatus status;
  final String? scanId;
  final String? rootPath;
  final String? displayRootPath;
  final LibraryTaskKind? taskKind;
  final LibraryScanPublication publication;
  final List<LibraryIssue> recentIssues;
  final int visitedEntries;
  final int stagedAssetCount;
  final LibraryScanPhase scanPhase;
  final int validatedAssetCount;
  final int validationAssetCount;
  final int issueCount;
  final int? itemLimit;
  final int? entryLimit;
  final bool isScanLimited;
  final bool isResumingScan;
  final String? errorMessage;

  bool get isScanning =>
      status == LibraryStatus.scanning ||
      status == LibraryStatus.pausing ||
      status == LibraryStatus.cancelling;

  bool get hasRetainedScan =>
      status == LibraryStatus.paused || status == LibraryStatus.discarding;

  bool get blocksExecution =>
      isScanning ||
      status == LibraryStatus.choosingDirectory ||
      status == LibraryStatus.refreshing;

  bool get hasFeedback => taskKind != null || scanId != null;

  LibraryPrimaryScanSnapshot copyWith({
    LibraryStatus? status,
    Object? scanId = _unchanged,
    Object? rootPath = _unchanged,
    Object? displayRootPath = _unchanged,
    Object? taskKind = _unchanged,
    LibraryScanPublication? publication,
    List<LibraryIssue>? recentIssues,
    int? visitedEntries,
    int? stagedAssetCount,
    LibraryScanPhase? scanPhase,
    int? validatedAssetCount,
    int? validationAssetCount,
    int? issueCount,
    Object? itemLimit = _unchanged,
    Object? entryLimit = _unchanged,
    bool? isScanLimited,
    bool? isResumingScan,
    Object? errorMessage = _unchanged,
  }) {
    return LibraryPrimaryScanSnapshot(
      status: status ?? this.status,
      scanId: scanId == _unchanged ? this.scanId : scanId as String?,
      rootPath: rootPath == _unchanged ? this.rootPath : rootPath as String?,
      displayRootPath: displayRootPath == _unchanged
          ? this.displayRootPath
          : displayRootPath as String?,
      taskKind: taskKind == _unchanged
          ? this.taskKind
          : taskKind as LibraryTaskKind?,
      publication: publication ?? this.publication,
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
      isScanLimited: isScanLimited ?? this.isScanLimited,
      isResumingScan: isResumingScan ?? this.isResumingScan,
      errorMessage: errorMessage == _unchanged
          ? this.errorMessage
          : errorMessage as String?,
    );
  }

  static const Object _unchanged = Object();
}
