import "dart:async";

import "package:flutter/foundation.dart";

import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_catalog.dart";
import "library_catalog_publication.dart";
import "library_page_operation.dart";
import "library_query_refresh.dart";
import "library_query_snapshot_reader.dart";

export "library_query_refresh.dart" show LibraryQueryUpdateOutcome;

const _timeNavigationRetryDelay = Duration(milliseconds: 120);
const _maxVisibleRangePageLoads = 2;
const _retainedDetailHighWatermark = 5000;
const _retainedDetailLowWatermark = 3500;

class _RetainedCatalogPage {
  const _RetainedCatalogPage({
    required this.assets,
    required this.startItemOffset,
    required this.previousCursor,
    required this.nextCursor,
  });

  final List<LibraryAsset> assets;
  final int startItemOffset;
  final LibraryCatalogCursor? previousCursor;
  final LibraryCatalogCursor? nextCursor;
}

class _PendingTimeNavigation {
  _PendingTimeNavigation({
    required this.generation,
    required this.query,
    required this.timeline,
    required this.anchor,
    required this.globalItemOffset,
  });

  final int generation;
  final LibraryGalleryQuery query;
  final LibraryTimeline timeline;
  final LibraryTimeAnchor anchor;
  final int globalItemOffset;
  final Completer<bool> completion = Completer<bool>();
}

typedef _VisibleRangeRequest = ({
  int generation,
  String queryId,
  BigInt? revision,
  int start,
  int end,
});

class LibraryViewportController {
  LibraryViewportController(
    this._catalog,
    this._readState,
    this._publishState,
    this._isHostDisposed,
    this._retainPreviewPending,
  );

  final LibraryCatalog _catalog;
  final LibraryState Function() _readState;
  final void Function(LibraryState) _publishState;
  final bool Function() _isHostDisposed;
  final void Function(Iterable<String>) _retainPreviewPending;
  final _publications = LibraryCatalogPublicationCoordinator();
  final _pages = LibraryPageOperation();
  late final _queryRefresh = LibraryQueryRefreshCoordinator(_publications);
  late final _querySnapshots = LibraryQuerySnapshotReader(_catalog);

  _PendingTimeNavigation? _pendingTimeNavigation;
  _PendingTimeNavigation? _activeTimeNavigation;
  _PendingTimeNavigation? _timeNavigationOwner;
  Timer? _timeNavigationRetryTimer;
  bool _isRunningTimeNavigation = false;
  int _timeNavigationGeneration = 0;
  int? _loadingTimeNavigationGeneration;
  int? _visibleRangeTimeNavigationGeneration;
  int _queryGeneration = 0;
  int _publicationGeneration = 0;
  _VisibleRangeRequest? _pendingVisibleRange;
  _VisibleRangeRequest? _activeVisibleRange;
  bool _isEnsuringVisibleRange = false;
  bool _isVisibleRangeDrainScheduled = false;
  final List<_RetainedCatalogPage> _retainedCatalogPages = [];
  LibraryState? _queryTransitionBaseState;
  int? _queryTransitionGeneration;
  bool _isDisposed = false;

  LibraryState get _state => _readState();

  set _state(LibraryState value) => _publishState(value);

  bool get _cannotPublish => _isDisposed || _isHostDisposed();

  void seed(LibraryState state) {
    _resetRetainedCatalogPages(
      assets: state.assets,
      startItemOffset: state.windowStartItemOffset,
      previousCursor: state.previousCursor,
      nextCursor: state.nextCursor,
    );
  }

  void dispose() {
    if (_isDisposed) {
      return;
    }
    _isDisposed = true;
    _queryRefresh.dispose();
    _publications.dispose();
    _timeNavigationRetryTimer?.cancel();
    _timeNavigationRetryTimer = null;
    _pendingVisibleRange = null;
    _activeVisibleRange = null;
    final pending = _pendingTimeNavigation;
    _pendingTimeNavigation = null;
    if (pending != null && !pending.completion.isCompleted) {
      pending.completion.complete(false);
    }
    final active = _activeTimeNavigation;
    _activeTimeNavigation = null;
    _timeNavigationOwner = null;
    if (active != null && !active.completion.isCompleted) {
      active.completion.complete(false);
    }
  }

  void supersedeExternalRequests() {
    _queryRefresh.invalidate();
    _queryTransitionBaseState = null;
    _queryTransitionGeneration = null;
    _publicationGeneration += 1;
    _supersedeQueryWindowRequests();
    if (!_cannotPublish && _state.isRefreshingQuery) {
      _state = _state.copyWith(
        queryActivity: const LibraryQueryIdle(),
        isLoadingTimeline: false,
      );
    }
  }

  LibraryCatalogPublicationLease? reserveCatalogPublication() {
    final lease = _publications.reserve();
    if (lease != null) {
      supersedeExternalRequests();
    }
    return lease;
  }

  void releaseCatalogPublication(LibraryCatalogPublicationLease lease) {
    _publications.release(lease);
  }

  Future<bool> refreshCurrentQuery() => _queryRefresh.refreshCommitted(
    () => _updateQueryWithOutcome(
      _state.query,
      forceRefresh: true,
      showRefreshingStatus: false,
    ),
  );

  Future<bool> updateQuery(
    LibraryGalleryQuery query, {
    String? anchorLocationId,
    String? anchorAssetId,
    int? fallbackGlobalItemIndex,
    bool forceRefresh = false,
    BigInt? minimumCatalogRevision,
    bool showRefreshingStatus = true,
  }) async {
    final outcome = await _queryRefresh.runUser(
      () => _updateQueryWithOutcome(
        query,
        anchorLocationId: anchorLocationId,
        anchorAssetId: anchorAssetId,
        fallbackGlobalItemIndex: fallbackGlobalItemIndex,
        forceRefresh: forceRefresh,
        minimumCatalogRevision: minimumCatalogRevision,
        showRefreshingStatus: showRefreshingStatus,
      ),
    );
    return outcome == LibraryQueryUpdateOutcome.applied;
  }

  Future<LibraryQueryUpdateOutcome> _updateQueryWithOutcome(
    LibraryGalleryQuery query, {
    String? anchorLocationId,
    String? anchorAssetId,
    int? fallbackGlobalItemIndex,
    bool forceRefresh = false,
    BigInt? minimumCatalogRevision,
    bool showRefreshingStatus = true,
  }) async {
    if (_cannotPublish) {
      return LibraryQueryUpdateOutcome.superseded;
    }
    if (_publications.isReserved) {
      return LibraryQueryUpdateOutcome.busy;
    }
    final normalized = query.copyWith(
      folderRelativePath: query.folderRelativePath
          ?.replaceAll("\\", "/")
          .replaceAll(RegExp(r"^/+|/+$"), ""),
      searchText: query.searchText.trim(),
    );
    final doesTransitionOwnGeneration =
        _queryTransitionBaseState != null &&
        _queryTransitionGeneration == _publicationGeneration;
    if (_queryTransitionBaseState != null && !doesTransitionOwnGeneration) {
      _queryTransitionBaseState = null;
      _queryTransitionGeneration = null;
    }
    if (!forceRefresh &&
        normalized == _state.query &&
        _queryTransitionBaseState == null) {
      return LibraryQueryUpdateOutcome.applied;
    }
    if (!forceRefresh &&
        normalized == _state.query &&
        _queryTransitionBaseState != null) {
      _supersedeQueryWindowRequests();
      _publicationGeneration += 1;
      final baseState = _queryTransitionBaseState!;
      _queryTransitionBaseState = null;
      _queryTransitionGeneration = null;
      _state = baseState;
      return LibraryQueryUpdateOutcome.applied;
    }
    final hasVisibleRangeRequest =
        _pendingVisibleRange != null || _activeVisibleRange != null;
    if (_state.status == LibraryStatus.choosingDirectory ||
        _state.isScanning ||
        (_state.isLoadingTimeAnchor && !hasVisibleRangeRequest)) {
      return LibraryQueryUpdateOutcome.busy;
    }
    _supersedeQueryWindowRequests();
    final priorGeneration = _publicationGeneration;
    final isContinuingTransition =
        _queryTransitionBaseState != null &&
        _queryTransitionGeneration == priorGeneration;
    final requestGeneration = ++_publicationGeneration;
    if (!isContinuingTransition) {
      _queryTransitionBaseState = _state;
    }
    _queryTransitionGeneration = requestGeneration;
    _state = _state.copyWith(
      queryActivity: showRefreshingStatus
          ? LibraryQueryLoading(normalized)
          : _state.queryActivity,
      isLoadingTimeline: true,
      pageErrorMessage: null,
      previousPageErrorMessage: null,
      timeNavigationErrorMessage: null,
      errorMessage: null,
    );
    try {
      final result = await _querySnapshots.load(
        query: normalized,
        anchor: anchorLocationId == null
            ? null
            : LibraryQueryAnchor(
                requestedLocationId: anchorLocationId,
                assetId: anchorAssetId,
                fallbackGlobalItemIndex: fallbackGlobalItemIndex ?? 0,
              ),
        minimumRevision: minimumCatalogRevision,
      );
      final snapshot = result.snapshot;
      final timeline = result.timeline;
      if (_cannotPublish || requestGeneration != _publicationGeneration) {
        return LibraryQueryUpdateOutcome.superseded;
      }
      final baseState = _queryTransitionBaseState ?? _state;
      final windowStart =
          snapshot.queryAnchorResolution?.windowStartItemOffset ?? 0;
      _resetRetainedCatalogPages(
        assets: snapshot.assets,
        startItemOffset: windowStart,
        previousCursor: snapshot.previousCursor,
        nextCursor: snapshot.nextCursor,
      );
      _state = LibraryState.fromSnapshot(snapshot, query: normalized).copyWith(
        queryActivity: showRefreshingStatus
            ? const LibraryQueryIdle()
            : baseState.queryActivity,
        status: baseState.taskKind == LibraryTaskKind.remove
            ? baseState.status
            : null,
        taskKind: baseState.taskKind == LibraryTaskKind.remove
            ? LibraryTaskKind.remove
            : null,
        removingRootId: baseState.removingRootId,
        removingRootDisplayPath: baseState.removingRootDisplayPath,
        isRemovalCommitted: baseState.isRemovalCommitted,
        completedRemovalRootId: baseState.completedRemovalRootId,
        rootRemovalCompletionSequence: baseState.rootRemovalCompletionSequence,
        windowStartItemOffset: windowStart,
        timeline: timeline,
        activeTimeAnchor: null,
        isLoadingTimeline: false,
        isLoadingTimeAnchor: false,
        pageErrorMessage: null,
        previousPageErrorMessage: null,
        timeNavigationErrorMessage: null,
        errorMessage: baseState.taskKind == LibraryTaskKind.remove
            ? baseState.errorMessage
            : null,
      );
      _queryTransitionBaseState = null;
      _queryTransitionGeneration = null;
      return LibraryQueryUpdateOutcome.applied;
    } on Object catch (error) {
      final wasSuperseded =
          _cannotPublish || requestGeneration != _publicationGeneration;
      if (kDebugMode) {
        final code = error is LibraryCatalogFailure
            ? error.code
            : error.runtimeType.toString();
        debugPrint(
          "[Ame query] outcome=${wasSuperseded ? 'superseded' : 'failed'} code=$code",
        );
      }
      if (wasSuperseded) {
        return LibraryQueryUpdateOutcome.superseded;
      }
      final baseState = _queryTransitionBaseState ?? _state;
      _queryTransitionBaseState = null;
      _queryTransitionGeneration = null;
      _state = baseState.copyWith(
        queryActivity: showRefreshingStatus
            ? LibraryQueryFailed(
                requestedQuery: normalized,
                message: error.toString(),
              )
            : baseState.queryActivity,
        isLoadingTimeline: false,
      );
      return LibraryQueryUpdateOutcome.failed;
    }
  }

  Future<LibraryQueryUpdateOutcome> refreshFromSynchronization({
    required BigInt catalogRevision,
    String? anchorLocationId,
    String? anchorAssetId,
    int? fallbackGlobalItemIndex,
  }) {
    return _queryRefresh.runPassive(
      () => _updateQueryWithOutcome(
        _state.query,
        anchorLocationId: anchorLocationId,
        anchorAssetId: anchorAssetId,
        fallbackGlobalItemIndex: fallbackGlobalItemIndex,
        forceRefresh: true,
        minimumCatalogRevision: catalogRevision,
        showRefreshingStatus: false,
      ),
    );
  }

  Future<void> loadNextPage() async {
    await _loadPage(LibraryPageDirection.next);
  }

  Future<bool> loadPreviousPage() => _loadPage(LibraryPageDirection.previous);

  Future<bool> _loadPage(LibraryPageDirection direction) {
    final state = _state;
    final generation = _publicationGeneration;
    final cursor = direction == LibraryPageDirection.next
        ? state.nextCursor
        : state.previousCursor;
    return _pages.run(generation, cursor, () {
      if (!_canPublishGeneration(generation)) {
        return Future.value(false);
      }
      final current = _state;
      final currentCursor = direction == LibraryPageDirection.next
          ? current.nextCursor
          : current.previousCursor;
      if (state.query != current.query ||
          state.queryId != current.queryId ||
          state.catalogRevision != current.catalogRevision ||
          !identical(cursor, currentCursor)) {
        return Future.value(false);
      }
      return direction == LibraryPageDirection.next
          ? _loadNextPage()
          : _loadPreviousPage();
    }, direction: direction);
  }

  Future<bool> _loadNextPage() async {
    final cursor = _state.nextCursor;
    if (cursor == null ||
        _state.isBusy ||
        _state.isLoadingPage ||
        _state.isLoadingPreviousPage ||
        _state.isLoadingTimeAnchor) {
      return false;
    }

    final generation = _publicationGeneration;
    _state = _state.copyWith(isLoadingPage: true, pageErrorMessage: null);
    try {
      final snapshot = await _catalog.load(
        maxItems: libraryCatalogWindow,
        query: _state.query,
        after: cursor,
      );
      if (!_canPublishGeneration(generation)) {
        return false;
      }
      if (snapshot.revision != _state.catalogRevision ||
          snapshot.queryId != _state.queryId) {
        throw const LibraryCatalogFailure(
          code: "catalog_cursor_stale",
          message: "The gallery query changed while loading another window",
        );
      }
      _ensureRetainedCatalogPagesCurrent();
      final existingIds = {for (final asset in _state.assets) asset.locationId};
      _mergeRetainedCatalogAssetUpdates(snapshot.assets);
      final addedAssets = [
        for (final asset in snapshot.assets)
          if (!existingIds.contains(asset.locationId)) asset,
      ];
      if (addedAssets.isNotEmpty) {
        _retainedCatalogPages.add(
          _RetainedCatalogPage(
            assets: List.unmodifiable(addedAssets),
            startItemOffset:
                _state.windowStartItemOffset + _state.assets.length,
            previousCursor: snapshot.previousCursor,
            nextCursor: snapshot.nextCursor,
          ),
        );
      } else if (_retainedCatalogPages.isNotEmpty) {
        final lastIndex = _retainedCatalogPages.length - 1;
        final lastPage = _retainedCatalogPages[lastIndex];
        _retainedCatalogPages[lastIndex] = _RetainedCatalogPage(
          assets: lastPage.assets,
          startItemOffset: lastPage.startItemOffset,
          previousCursor: lastPage.previousCursor,
          nextCursor: snapshot.nextCursor,
        );
      }
      _trimRetainedCatalogPages(trimLeading: true);
      final retainedAssets = _retainedCatalogAssets();
      final firstPage = _retainedCatalogPages.first;
      _state = _state.copyWith(
        roots: snapshot.roots,
        assets: retainedAssets,
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
        windowStartItemOffset: firstPage.startItemOffset,
        previousCursor: firstPage.previousCursor,
        nextCursor: snapshot.nextCursor,
        isLoadingPage: false,
      );
      return addedAssets.isNotEmpty;
    } on LibraryCatalogFailure catch (error) {
      if (!_canPublishGeneration(generation)) {
        return false;
      }
      if (error.code == "catalog_cursor_stale") {
        try {
          await reloadFirstCatalogPage();
        } on Object catch (refreshError) {
          if (_canPublishGeneration(generation)) {
            _state = _state.copyWith(
              isLoadingPage: false,
              pageErrorMessage: refreshError.toString(),
            );
          }
        }
        return false;
      }
      _state = _state.copyWith(
        isLoadingPage: false,
        pageErrorMessage: error.toString(),
      );
      return false;
    } on Object catch (error) {
      if (!_canPublishGeneration(generation)) {
        return false;
      }
      _state = _state.copyWith(
        isLoadingPage: false,
        pageErrorMessage: error.toString(),
      );
      return false;
    }
  }

  Future<bool> _loadPreviousPage() async {
    final cursor = _state.previousCursor;
    if (cursor == null ||
        _state.isBusy ||
        _state.isLoadingPage ||
        _state.isLoadingPreviousPage ||
        _state.isLoadingTimeAnchor) {
      return false;
    }

    final generation = _publicationGeneration;
    _state = _state.copyWith(
      isLoadingPreviousPage: true,
      previousPageErrorMessage: null,
    );
    try {
      final snapshot = await _catalog.load(
        maxItems: libraryCatalogWindow,
        query: _state.query,
        before: cursor,
      );
      if (!_canPublishGeneration(generation)) {
        return false;
      }
      if (snapshot.revision != _state.catalogRevision ||
          snapshot.queryId != _state.queryId) {
        throw const LibraryCatalogFailure(
          code: "catalog_cursor_stale",
          message: "The gallery changed while loading the previous window",
        );
      }
      _ensureRetainedCatalogPagesCurrent();
      final existingIds = {for (final asset in _state.assets) asset.locationId};
      _mergeRetainedCatalogAssetUpdates(snapshot.assets);
      final addedAssets = [
        for (final asset in snapshot.assets)
          if (!existingIds.contains(asset.locationId)) asset,
      ];
      final addedItemCount = addedAssets.length;
      if (addedAssets.isNotEmpty) {
        _retainedCatalogPages.insert(
          0,
          _RetainedCatalogPage(
            assets: List.unmodifiable(addedAssets),
            startItemOffset: (_state.windowStartItemOffset - addedItemCount)
                .clamp(0, _state.timeline?.totalItems ?? 0)
                .toInt(),
            previousCursor: snapshot.previousCursor,
            nextCursor: snapshot.nextCursor,
          ),
        );
      } else if (_retainedCatalogPages.isNotEmpty) {
        final firstPage = _retainedCatalogPages.first;
        _retainedCatalogPages[0] = _RetainedCatalogPage(
          assets: firstPage.assets,
          startItemOffset: firstPage.startItemOffset,
          previousCursor: snapshot.previousCursor,
          nextCursor: firstPage.nextCursor,
        );
      }
      _trimRetainedCatalogPages(trimLeading: false);
      final retainedAssets = _retainedCatalogAssets();
      final firstPage = _retainedCatalogPages.first;
      final lastPage = _retainedCatalogPages.last;
      _state = _state.copyWith(
        roots: snapshot.roots,
        assets: retainedAssets,
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
        windowStartItemOffset: firstPage.startItemOffset,
        previousCursor: firstPage.previousCursor,
        nextCursor: lastPage.nextCursor,
        isLoadingPreviousPage: false,
      );
      return snapshot.assets.isNotEmpty;
    } on LibraryCatalogFailure catch (error) {
      if (!_canPublishGeneration(generation)) {
        return false;
      }
      if (error.code == "catalog_cursor_stale") {
        try {
          await reloadFirstCatalogPage();
        } on Object catch (refreshError) {
          if (_canPublishGeneration(generation)) {
            _state = _state.copyWith(
              isLoadingPreviousPage: false,
              previousPageErrorMessage: refreshError.toString(),
            );
          }
        }
        return false;
      }
      _state = _state.copyWith(
        isLoadingPreviousPage: false,
        previousPageErrorMessage: error.toString(),
      );
      return false;
    } on Object catch (error) {
      if (_canPublishGeneration(generation)) {
        _state = _state.copyWith(
          isLoadingPreviousPage: false,
          previousPageErrorMessage: error.toString(),
        );
      }
      return false;
    }
  }

  Future<bool> jumpToTime(LibraryTimeBucket bucket, {int itemOffset = 0}) {
    return _requestTimeNavigation(
      bucket,
      itemOffset: itemOffset,
      ownsVisibleRange: true,
    );
  }

  Future<bool> prefetchTime(LibraryTimeBucket bucket, {int itemOffset = 0}) {
    return _requestTimeNavigation(
      bucket,
      itemOffset: itemOffset,
      ownsVisibleRange: false,
    );
  }

  Future<bool> _requestTimeNavigation(
    LibraryTimeBucket bucket, {
    required int itemOffset,
    required bool ownsVisibleRange,
  }) {
    final timeline = _state.timeline;
    if (timeline == null) {
      return Future.value(false);
    }
    if (!ownsVisibleRange && _hasCompatibleTimeNavigationOwner()) {
      return Future.value(false);
    }
    if (ownsVisibleRange) {
      _pendingVisibleRange = null;
    }
    final anchor = LibraryTimeAnchor(
      revision: timeline.revision,
      queryId: timeline.queryId,
      monthKey: bucket.monthKey,
      itemOffset: itemOffset.clamp(
        0,
        bucket.itemCount > 0 ? bucket.itemCount - 1 : 0,
      ),
    );
    final globalItemOffset = _globalItemOffsetForAnchor(
      timeline,
      bucket,
      anchor.itemOffset,
    );
    final loadedStart = _state.windowStartItemOffset;
    final loadedEnd = loadedStart + _state.assets.length;
    final needsVisibleRangeLoading =
        ownsVisibleRange &&
        (globalItemOffset < loadedStart || globalItemOffset >= loadedEnd);
    final pending = _pendingTimeNavigation;
    if (pending != null &&
        _matchesTimeNavigationTarget(
          pending,
          timeline,
          _state.query,
          globalItemOffset,
        )) {
      if (ownsVisibleRange) {
        _timeNavigationOwner = pending;
        _setTimeNavigationVisibleRangeLoading(
          pending,
          needsVisibleRangeLoading,
        );
      }
      return pending.completion.future;
    }
    final active = _activeTimeNavigation;
    if (active != null &&
        active.generation == _timeNavigationGeneration &&
        _matchesTimeNavigationTarget(
          active,
          timeline,
          _state.query,
          globalItemOffset,
        )) {
      if (ownsVisibleRange) {
        _timeNavigationOwner = active;
        _setTimeNavigationVisibleRangeLoading(active, needsVisibleRangeLoading);
      }
      return active.completion.future;
    }
    final generation = ++_timeNavigationGeneration;
    final request = _PendingTimeNavigation(
      generation: generation,
      query: _state.query,
      timeline: timeline,
      anchor: anchor,
      globalItemOffset: globalItemOffset,
    );
    final previousPending = _pendingTimeNavigation;
    _pendingTimeNavigation = request;
    if (ownsVisibleRange) {
      _timeNavigationOwner = request;
      _setTimeNavigationVisibleRangeLoading(request, needsVisibleRangeLoading);
    }
    if (previousPending != null && !previousPending.completion.isCompleted) {
      previousPending.completion.complete(false);
    }
    _scheduleTimeNavigationDrain();
    return request.completion.future;
  }

  bool _matchesTimeNavigationTarget(
    _PendingTimeNavigation request,
    LibraryTimeline timeline,
    LibraryGalleryQuery query,
    int globalItemOffset,
  ) {
    return request.query == query &&
        request.timeline.revision == timeline.revision &&
        request.timeline.queryId == timeline.queryId &&
        request.globalItemOffset == globalItemOffset;
  }

  void _scheduleTimeNavigationDrain() {
    if (_cannotPublish || _isRunningTimeNavigation) {
      return;
    }
    _timeNavigationRetryTimer?.cancel();
    _timeNavigationRetryTimer = null;
    unawaited(Future<void>.microtask(_drainTimeNavigation));
  }

  Future<void> _drainTimeNavigation() async {
    if (_cannotPublish || _isRunningTimeNavigation) {
      return;
    }
    final request = _pendingTimeNavigation;
    if (request == null) {
      return;
    }
    if (!_isCompatibleTimeNavigation(request)) {
      _pendingTimeNavigation = null;
      _releaseTimeNavigationVisibleRangeLoading(request);
      if (identical(_timeNavigationOwner, request)) {
        _timeNavigationOwner = null;
      }
      if (!request.completion.isCompleted) {
        request.completion.complete(false);
      }
      _scheduleTimeNavigationDrain();
      return;
    }
    if (_isTimeNavigationBlocked) {
      _timeNavigationRetryTimer ??= Timer(
        _timeNavigationRetryDelay,
        _scheduleTimeNavigationDrain,
      );
      return;
    }

    _pendingTimeNavigation = null;
    _isRunningTimeNavigation = true;
    _activeTimeNavigation = request;
    try {
      final didLoad = await _loadTimeNavigation(request);
      final isLatest = request.generation == _timeNavigationGeneration;
      if (!didLoad && identical(_timeNavigationOwner, request)) {
        _timeNavigationOwner = null;
      }
      if (!request.completion.isCompleted) {
        request.completion.complete(didLoad && isLatest);
      }
    } finally {
      _releaseTimeNavigationVisibleRangeLoading(request);
      if (identical(_activeTimeNavigation, request)) {
        _activeTimeNavigation = null;
      }
      _isRunningTimeNavigation = false;
      _scheduleTimeNavigationDrain();
    }
  }

  bool get _isTimeNavigationBlocked =>
      _state.isProcessing ||
      _state.isLoadingPage ||
      _state.isLoadingPreviousPage ||
      _state.isLoadingTimeAnchor;

  bool _isCompatibleTimeNavigation(_PendingTimeNavigation request) {
    final timeline = _state.timeline;
    return timeline != null &&
        timeline.revision == request.timeline.revision &&
        timeline.queryId == request.timeline.queryId &&
        _state.query == request.query;
  }

  Future<bool> _loadTimeNavigation(_PendingTimeNavigation request) async {
    final generation = ++_publicationGeneration;
    _loadingTimeNavigationGeneration = request.generation;
    _state = _state.copyWith(
      isLoadingTimeAnchor: true,
      timeNavigationErrorMessage: null,
    );
    try {
      final snapshot = await _catalog.loadAtTime(
        maxItems: libraryTimelineWindow,
        query: request.query,
        anchor: request.anchor,
      );
      if (!_canPublishTimeNavigation(request, generation)) {
        return false;
      }
      if (snapshot.revision != request.timeline.revision ||
          snapshot.queryId != request.timeline.queryId) {
        throw const LibraryCatalogFailure(
          code: "catalog_cursor_stale",
          message: "The catalog changed while navigating the timeline",
        );
      }
      _retainPreviewPending(snapshot.assets.map((asset) => asset.locationId));
      _resetRetainedCatalogPages(
        assets: snapshot.assets,
        startItemOffset: request.globalItemOffset,
        previousCursor: snapshot.previousCursor,
        nextCursor: snapshot.nextCursor,
      );
      _state = _state.copyWith(
        roots: snapshot.roots,
        assets: snapshot.assets,
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
        windowStartItemOffset: request.globalItemOffset,
        previousCursor: snapshot.previousCursor,
        nextCursor: snapshot.nextCursor,
        activeTimeAnchor: request.anchor,
        isLoadingTimeAnchor: false,
        pageErrorMessage: null,
      );
      return true;
    } on LibraryCatalogFailure catch (error) {
      if (!_canPublishTimeNavigation(request, generation)) {
        return false;
      }
      if (error.code == "catalog_cursor_stale") {
        try {
          await reloadFirstCatalogPage();
        } on Object catch (refreshError) {
          if (_canPublishGeneration(generation)) {
            _state = _state.copyWith(
              isLoadingTimeAnchor: false,
              timeNavigationErrorMessage: refreshError.toString(),
            );
          }
        }
        return false;
      }
      _state = _state.copyWith(
        isLoadingTimeAnchor: false,
        timeNavigationErrorMessage: error.toString(),
      );
      return false;
    } on Object catch (error) {
      if (_canPublishTimeNavigation(request, generation)) {
        _state = _state.copyWith(
          isLoadingTimeAnchor: false,
          timeNavigationErrorMessage: error.toString(),
        );
      }
      return false;
    } finally {
      _releaseTimeNavigationLoading(request);
    }
  }

  bool _canPublishTimeNavigation(
    _PendingTimeNavigation request,
    int generation,
  ) {
    return _canPublishGeneration(generation) &&
        request.generation == _timeNavigationGeneration &&
        _isCompatibleTimeNavigation(request);
  }

  void _releaseTimeNavigationLoading(_PendingTimeNavigation request) {
    if (_loadingTimeNavigationGeneration != request.generation) {
      return;
    }
    _loadingTimeNavigationGeneration = null;
    if (!_cannotPublish && _state.isLoadingTimeAnchor) {
      _state = _state.copyWith(isLoadingTimeAnchor: false);
    }
  }

  void cancelTimeNavigation() {
    if (_cannotPublish) {
      return;
    }
    final pending = _pendingTimeNavigation;
    final active = _activeTimeNavigation;
    final owner = _timeNavigationOwner;
    final hasPendingRequest =
        pending != null && !pending.completion.isCompleted;
    final hasActiveRequest = active != null && !active.completion.isCompleted;
    if (!hasPendingRequest && !hasActiveRequest && owner == null) {
      if (_state.activeTimeAnchor != null) {
        _state = _state.copyWith(activeTimeAnchor: null);
      }
      return;
    }
    _timeNavigationGeneration += 1;
    _pendingTimeNavigation = null;
    _timeNavigationOwner = null;
    _visibleRangeTimeNavigationGeneration = null;
    _timeNavigationRetryTimer?.cancel();
    _timeNavigationRetryTimer = null;
    if (pending != null && !pending.completion.isCompleted) {
      pending.completion.complete(false);
    }
    if (active != null && !active.completion.isCompleted) {
      active.completion.complete(false);
    }
    _loadingTimeNavigationGeneration = null;
    if (_state.isLoadingTimeAnchor || _state.activeTimeAnchor != null) {
      _state = _state.copyWith(
        activeTimeAnchor: null,
        isLoadingTimeAnchor: false,
      );
    }
    _setVisibleRangeLoading(false);
  }

  void ensureVisibleRange({
    required int startItemOffset,
    required int endItemOffsetExclusive,
  }) {
    if (_cannotPublish ||
        _queryTransitionBaseState != null ||
        endItemOffsetExclusive <= startItemOffset) {
      return;
    }
    final totalItems = _state.timeline?.totalItems ?? 0;
    if (totalItems <= 0) {
      return;
    }
    final start = startItemOffset.clamp(0, totalItems - 1).toInt();
    final end = endItemOffsetExclusive.clamp(start + 1, totalItems).toInt();
    final request = (
      generation: _queryGeneration,
      queryId: _state.queryId,
      revision: _state.catalogRevision,
      start: start,
      end: end,
    );
    if (!_ownsVisibleRangeRequest(request) ||
        !_visibleRangeRetainsNavigationOwner(request)) {
      return;
    }
    if (_timeNavigationOwner == null) {
      _retainPassiveTimeNavigationForVisibleRange(request);
    }
    final loadedStart = _state.windowStartItemOffset;
    final loadedEnd = loadedStart + _state.assets.length;
    if (request.start >= loadedStart && request.end <= loadedEnd) {
      _pendingVisibleRange = null;
      _setVisibleRangeLoading(false);
      return;
    }
    if (_pendingVisibleRange == request) {
      return;
    }
    _pendingVisibleRange = request;
    _setVisibleRangeLoading(true);
    _scheduleVisibleRangeDrain();
  }

  bool _ownsVisibleRangeRequest(_VisibleRangeRequest request) {
    return !_cannotPublish &&
        request.generation == _queryGeneration &&
        request.queryId == _state.queryId &&
        request.revision == _state.catalogRevision;
  }

  bool _visibleRangeRetainsNavigationOwner(_VisibleRangeRequest request) {
    final owner = _timeNavigationOwner;
    if (owner == null) {
      return true;
    }
    if (!_isCompatibleTimeNavigation(owner)) {
      _timeNavigationOwner = null;
      return true;
    }
    return owner.globalItemOffset >= request.start &&
        owner.globalItemOffset < request.end;
  }

  bool _hasCompatibleTimeNavigationOwner() {
    final owner = _timeNavigationOwner;
    if (owner == null) {
      return false;
    }
    if (_isCompatibleTimeNavigation(owner)) {
      return true;
    }
    _timeNavigationOwner = null;
    return false;
  }

  void _retainPassiveTimeNavigationForVisibleRange(
    _VisibleRangeRequest rangeRequest,
  ) {
    bool contains(_PendingTimeNavigation navigationRequest) {
      return navigationRequest.globalItemOffset >= rangeRequest.start &&
          navigationRequest.globalItemOffset < rangeRequest.end;
    }

    final pending = _pendingTimeNavigation;
    if (pending != null && !contains(pending)) {
      if (pending.generation == _timeNavigationGeneration) {
        _timeNavigationGeneration += 1;
      }
      _pendingTimeNavigation = null;
      if (!pending.completion.isCompleted) {
        pending.completion.complete(false);
      }
    }
    final active = _activeTimeNavigation;
    if (active != null &&
        !contains(active) &&
        active.generation == _timeNavigationGeneration) {
      _timeNavigationGeneration += 1;
    }
    if (_pendingTimeNavigation == null) {
      _timeNavigationRetryTimer?.cancel();
      _timeNavigationRetryTimer = null;
    }
  }

  void _scheduleVisibleRangeDrain() {
    if (_cannotPublish ||
        _isEnsuringVisibleRange ||
        _isVisibleRangeDrainScheduled ||
        _pendingVisibleRange == null) {
      return;
    }
    _isVisibleRangeDrainScheduled = true;
    unawaited(
      Future<void>.microtask(() {
        _isVisibleRangeDrainScheduled = false;
        return _drainVisibleRange();
      }),
    );
  }

  Future<void> _drainVisibleRange() async {
    _isVisibleRangeDrainScheduled = false;
    if (_cannotPublish || _isEnsuringVisibleRange) {
      return;
    }
    _isEnsuringVisibleRange = true;
    try {
      while (!_cannotPublish) {
        final request = _pendingVisibleRange;
        _pendingVisibleRange = null;
        if (request == null) {
          return;
        }
        if (!_ownsVisibleRangeRequest(request)) {
          continue;
        }
        _activeVisibleRange = request;
        try {
          await _loadVisibleRange(request);
        } finally {
          if (_activeVisibleRange == request) {
            _activeVisibleRange = null;
          }
        }
      }
    } finally {
      _isEnsuringVisibleRange = false;
      if (_pendingVisibleRange != null && !_cannotPublish) {
        _scheduleVisibleRangeDrain();
      } else {
        _setVisibleRangeLoading(false);
      }
    }
  }

  void _setVisibleRangeLoading(bool isLoading) {
    final shouldShowLoading =
        isLoading ||
        _pendingVisibleRange != null ||
        _activeVisibleRange != null ||
        _visibleRangeTimeNavigationGeneration != null;
    if (_cannotPublish || _state.isLoadingVisibleRange == shouldShowLoading) {
      return;
    }
    _state = _state.copyWith(isLoadingVisibleRange: shouldShowLoading);
  }

  void _setTimeNavigationVisibleRangeLoading(
    _PendingTimeNavigation request,
    bool isLoading,
  ) {
    _visibleRangeTimeNavigationGeneration = isLoading
        ? request.generation
        : null;
    _setVisibleRangeLoading(isLoading);
  }

  void _releaseTimeNavigationVisibleRangeLoading(
    _PendingTimeNavigation request,
  ) {
    if (_visibleRangeTimeNavigationGeneration != request.generation) {
      return;
    }
    _visibleRangeTimeNavigationGeneration = null;
    _setVisibleRangeLoading(false);
  }

  Future<void> _loadVisibleRange(_VisibleRangeRequest request) async {
    for (var attempt = 0; attempt < _maxVisibleRangePageLoads; attempt++) {
      if (_cannotPublish ||
          _state.assets.isEmpty ||
          !_ownsVisibleRangeRequest(request) ||
          !_visibleRangeRetainsNavigationOwner(request)) {
        return;
      }
      final loadedStart = _state.windowStartItemOffset;
      final loadedEnd = loadedStart + _state.assets.length;
      if (request.end <= loadedStart || request.start >= loadedEnd) {
        await _loadDisjointVisibleRange(request);
        if (!_ownsVisibleRangeRequest(request)) {
          return;
        }
        return;
      }
      if (request.start < loadedStart) {
        final previousStart = loadedStart;
        final didLoad = await loadPreviousPage();
        if (!_ownsVisibleRangeRequest(request) ||
            !didLoad ||
            _state.windowStartItemOffset >= previousStart) {
          return;
        }
        continue;
      }
      if (request.end > loadedEnd) {
        final previousEnd = loadedEnd;
        await loadNextPage();
        if (!_ownsVisibleRangeRequest(request)) {
          return;
        }
        final nextEnd = _state.windowStartItemOffset + _state.assets.length;
        if (nextEnd <= previousEnd) {
          return;
        }
        continue;
      }
      return;
    }
  }

  Future<void> _loadDisjointVisibleRange(_VisibleRangeRequest request) async {
    if (!_ownsVisibleRangeRequest(request)) {
      return;
    }
    final timeline = _state.timeline;
    if (timeline == null || timeline.buckets.isEmpty) {
      return;
    }
    final target = request.start
        .clamp(0, timeline.totalItems > 0 ? timeline.totalItems - 1 : 0)
        .toInt();
    var precedingItems = 0;
    for (final bucket in timeline.buckets) {
      final bucketEnd = precedingItems + bucket.itemCount;
      if (target < bucketEnd) {
        await _requestTimeNavigation(
          bucket,
          itemOffset: target - precedingItems,
          ownsVisibleRange: false,
        );
        if (!_ownsVisibleRangeRequest(request)) {
          return;
        }
        return;
      }
      precedingItems = bucketEnd;
    }
    final lastBucket = timeline.buckets.last;
    await _requestTimeNavigation(
      lastBucket,
      itemOffset: lastBucket.itemCount > 0 ? lastBucket.itemCount - 1 : 0,
      ownsVisibleRange: false,
    );
  }

  void publishCommittedRootRemovalProjection(LibraryRoot removedRoot) {
    final roots = List<LibraryRoot>.unmodifiable(
      _state.roots.where((root) => root.id != removedRoot.id),
    );
    final assets = List<LibraryAsset>.unmodifiable(
      _state.assets.where((asset) => asset.rootId != removedRoot.id),
    );
    final query = _state.query.rootId == removedRoot.id
        ? const LibraryGalleryQuery()
        : _state.query;
    _resetRetainedCatalogPages(
      assets: assets,
      startItemOffset: 0,
      previousCursor: null,
      nextCursor: null,
    );
    _retainPreviewPending(assets.map((asset) => asset.locationId));
    _state = _state.copyWith(
      rootPath: _state.rootPath == removedRoot.path ? null : _state.rootPath,
      displayRootPath: _state.displayRootPath == removedRoot.displayPath
          ? null
          : _state.displayRootPath,
      isRemovalCommitted: true,
      roots: roots,
      assets: assets,
      query: query,
      queryId: "",
      windowStartItemOffset: 0,
      previousCursor: null,
      nextCursor: null,
      timeline: null,
      activeTimeAnchor: null,
      queryAnchorResolution: null,
      isLoadingPage: false,
      isLoadingPreviousPage: false,
      isLoadingTimeline: roots.isNotEmpty,
      isLoadingTimeAnchor: false,
      isLoadingVisibleRange: false,
      pageErrorMessage: null,
      previousPageErrorMessage: null,
      timeNavigationErrorMessage: null,
      errorMessage: null,
    );
  }

  Future<bool> reloadCommittedRootRemovalFirstPage({
    required String removedRootId,
  }) async {
    final generation = _publicationGeneration;
    final query = _state.query;
    final snapshot = await _catalog.load(
      maxItems: libraryCatalogWindow,
      query: query,
    );
    if (snapshot.roots.any((root) => root.id == removedRootId)) {
      throw const LibraryCatalogFailure(
        code: "catalog_root_removal_not_visible",
        message: "The removed library root is still present in the catalog",
      );
    }
    if (!_canPublishGeneration(generation)) {
      return false;
    }
    final shouldLoadTimeline = snapshot.roots.isNotEmpty;
    _publishFirstCatalogPage(
      generation: generation,
      snapshot: snapshot,
      query: query,
      timeline: null,
      isLoadingTimeline: shouldLoadTimeline,
    );
    if (!shouldLoadTimeline ||
        !_canPublishBackgroundTimeline(
          generation: generation,
          query: query,
          revision: snapshot.revision,
          queryId: snapshot.queryId,
        )) {
      return true;
    }
    unawaited(
      _loadBackgroundTimeline(
        generation: generation,
        query: query,
        revision: snapshot.revision,
        queryId: snapshot.queryId,
      ),
    );
    return true;
  }

  Future<bool> reloadFirstCatalogPage() async {
    final generation = _publicationGeneration;
    final query = _state.query;
    final result = await _querySnapshots.load(query: query);
    final snapshot = result.snapshot;
    final timeline = result.timeline;
    if (!_canPublishGeneration(generation)) {
      return false;
    }
    _publishFirstCatalogPage(
      generation: generation,
      snapshot: snapshot,
      query: query,
      timeline: timeline,
      isLoadingTimeline: false,
    );
    return true;
  }

  Future<void> loadInitialTimeline() async {
    if (_state.roots.isEmpty || _state.timeline != null || _state.isBusy) {
      return;
    }
    final generation = _publicationGeneration;
    _state = _state.copyWith(
      isLoadingTimeline: true,
      timeNavigationErrorMessage: null,
    );
    try {
      final timeline = await _catalog.loadTimeline(_state.query);
      if (!_canPublishGeneration(generation)) {
        return;
      }
      if (timeline.revision != _state.catalogRevision ||
          timeline.queryId != _state.queryId) {
        await reloadFirstCatalogPage();
        return;
      }
      _state = _state.copyWith(timeline: timeline, isLoadingTimeline: false);
    } on Object catch (error) {
      if (_canPublishGeneration(generation)) {
        _state = _state.copyWith(
          isLoadingTimeline: false,
          timeNavigationErrorMessage: error.toString(),
        );
      }
    }
  }

  Future<void> _loadBackgroundTimeline({
    required int generation,
    required LibraryGalleryQuery query,
    required BigInt revision,
    required String queryId,
  }) async {
    try {
      final timeline = await _catalog.loadTimeline(query);
      if (!_canPublishBackgroundTimeline(
        generation: generation,
        query: query,
        revision: revision,
        queryId: queryId,
      )) {
        return;
      }
      if (timeline.revision != revision || timeline.queryId != queryId) {
        await _queryRefresh.refreshCommitted(() async {
          if (!_canPublishBackgroundTimeline(
            generation: generation,
            query: query,
            revision: revision,
            queryId: queryId,
          )) {
            return LibraryQueryUpdateOutcome.superseded;
          }
          return await reloadFirstCatalogPage()
              ? LibraryQueryUpdateOutcome.applied
              : LibraryQueryUpdateOutcome.superseded;
        });
        return;
      }
      _state = _state.copyWith(
        timeline: timeline,
        isLoadingTimeline: false,
        timeNavigationErrorMessage: null,
      );
    } on Object catch (error) {
      if (_canPublishBackgroundTimeline(
        generation: generation,
        query: query,
        revision: revision,
        queryId: queryId,
      )) {
        _state = _state.copyWith(
          isLoadingTimeline: false,
          timeNavigationErrorMessage: error.toString(),
        );
      }
    }
  }

  bool _canPublishBackgroundTimeline({
    required int generation,
    required LibraryGalleryQuery query,
    required BigInt revision,
    required String queryId,
  }) {
    return _canPublishGeneration(generation) &&
        _state.query == query &&
        _state.catalogRevision == revision &&
        _state.queryId == queryId;
  }

  void _publishFirstCatalogPage({
    required int generation,
    required LibrarySnapshot snapshot,
    required LibraryGalleryQuery query,
    required LibraryTimeline? timeline,
    required bool isLoadingTimeline,
  }) {
    if (!_canPublishGeneration(generation)) {
      return;
    }
    final previousState = _state;
    final wasRemoving = previousState.status == LibraryStatus.removing;
    _resetRetainedCatalogPages(
      assets: snapshot.assets,
      startItemOffset: 0,
      previousCursor: snapshot.previousCursor,
      nextCursor: snapshot.nextCursor,
    );
    _state = LibraryState.fromSnapshot(snapshot, query: query).copyWith(
      taskKind: previousState.taskKind == LibraryTaskKind.remove
          ? LibraryTaskKind.remove
          : null,
      removingRootId: previousState.removingRootId,
      removingRootDisplayPath: previousState.removingRootDisplayPath,
      isRemovalCommitted: previousState.isRemovalCommitted,
      completedRemovalRootId: previousState.completedRemovalRootId,
      rootRemovalCompletionSequence:
          previousState.rootRemovalCompletionSequence,
      status: wasRemoving ? LibraryStatus.removing : null,
      timeline: timeline,
      activeTimeAnchor: null,
      isLoadingTimeline: isLoadingTimeline,
      isLoadingTimeAnchor: false,
      timeNavigationErrorMessage: null,
    );
  }

  void _supersedeQueryWindowRequests() {
    _queryGeneration += 1;
    _pendingVisibleRange = null;
    _activeVisibleRange = null;
    _visibleRangeTimeNavigationGeneration = null;

    final pending = _pendingTimeNavigation;
    final active = _activeTimeNavigation;
    if (pending != null || active != null) {
      _timeNavigationGeneration += 1;
    }
    _pendingTimeNavigation = null;
    _timeNavigationOwner = null;
    _timeNavigationRetryTimer?.cancel();
    _timeNavigationRetryTimer = null;
    _loadingTimeNavigationGeneration = null;
    if (pending != null && !pending.completion.isCompleted) {
      pending.completion.complete(false);
    }
    if (active != null && !active.completion.isCompleted) {
      active.completion.complete(false);
    }

    if (!_cannotPublish &&
        (_state.isLoadingPage ||
            _state.isLoadingPreviousPage ||
            _state.isLoadingTimeAnchor ||
            _state.isLoadingVisibleRange)) {
      _state = _state.copyWith(
        isLoadingPage: false,
        isLoadingPreviousPage: false,
        isLoadingTimeAnchor: false,
        isLoadingVisibleRange: false,
      );
    }
  }

  bool _canPublishGeneration(int generation) {
    return !_cannotPublish && generation == _publicationGeneration;
  }

  void _resetRetainedCatalogPages({
    required List<LibraryAsset> assets,
    required int startItemOffset,
    required LibraryCatalogCursor? previousCursor,
    required LibraryCatalogCursor? nextCursor,
  }) {
    _retainedCatalogPages
      ..clear()
      ..addAll(
        assets.isEmpty
            ? const []
            : [
                _RetainedCatalogPage(
                  assets: List.unmodifiable(assets),
                  startItemOffset: startItemOffset,
                  previousCursor: previousCursor,
                  nextCursor: nextCursor,
                ),
              ],
      );
  }

  void _ensureRetainedCatalogPagesCurrent() {
    final retainedCount = _retainedCatalogPages.fold(
      0,
      (total, page) => total + page.assets.length,
    );
    if (_retainedCatalogPages.isNotEmpty &&
        retainedCount == _state.assets.length &&
        _retainedCatalogPages.first.startItemOffset ==
            _state.windowStartItemOffset) {
      return;
    }
    _resetRetainedCatalogPages(
      assets: _state.assets,
      startItemOffset: _state.windowStartItemOffset,
      previousCursor: _state.previousCursor,
      nextCursor: _state.nextCursor,
    );
  }

  void _mergeRetainedCatalogAssetUpdates(List<LibraryAsset> updates) {
    final replacements = {for (final asset in updates) asset.locationId: asset};
    if (replacements.isEmpty) {
      return;
    }
    for (var index = 0; index < _retainedCatalogPages.length; index++) {
      final page = _retainedCatalogPages[index];
      var didChange = false;
      final assets = [
        for (final asset in page.assets)
          replacements[asset.locationId] ?? asset,
      ];
      for (var assetIndex = 0; assetIndex < page.assets.length; assetIndex++) {
        if (!identical(page.assets[assetIndex], assets[assetIndex])) {
          didChange = true;
          break;
        }
      }
      if (didChange) {
        _retainedCatalogPages[index] = _RetainedCatalogPage(
          assets: List.unmodifiable(assets),
          startItemOffset: page.startItemOffset,
          previousCursor: page.previousCursor,
          nextCursor: page.nextCursor,
        );
      }
    }
  }

  void _trimRetainedCatalogPages({required bool trimLeading}) {
    var retainedCount = _retainedCatalogPages.fold(
      0,
      (total, page) => total + page.assets.length,
    );
    if (retainedCount <= _retainedDetailHighWatermark) {
      return;
    }
    while (_retainedCatalogPages.length > 1 &&
        retainedCount > _retainedDetailLowWatermark) {
      final removed = trimLeading
          ? _retainedCatalogPages.removeAt(0)
          : _retainedCatalogPages.removeLast();
      retainedCount -= removed.assets.length;
    }
  }

  List<LibraryAsset> _retainedCatalogAssets() {
    return List.unmodifiable([
      for (final page in _retainedCatalogPages) ...page.assets,
    ]);
  }

  static int _globalItemOffsetForAnchor(
    LibraryTimeline timeline,
    LibraryTimeBucket selectedBucket,
    int itemOffset,
  ) {
    var precedingItems = 0;
    for (final bucket in timeline.buckets) {
      if (identical(bucket, selectedBucket) ||
          bucket.monthKey == selectedBucket.monthKey) {
        return (precedingItems + itemOffset)
            .clamp(0, timeline.totalItems)
            .toInt();
      }
      precedingItems += bucket.itemCount;
    }
    return 0;
  }
}
