import "dart:async";

import "package:flutter/foundation.dart";

import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_browse_admission.dart";
import "library_catalog.dart";
import "library_catalog_publication.dart";
import "library_page_operation.dart";
import "library_query_projection.dart";
import "library_query_refresh.dart";
import "library_query_snapshot_reader.dart";
import "library_query_transition.dart";
import "library_retained_catalog_window.dart";
import "library_root_consolidation.dart";
import "library_root_removal_page_reader.dart";
import "library_time_navigation_requests.dart";
import "library_time_snapshot_reader.dart";

export "library_query_refresh.dart" show LibraryQueryUpdateOutcome;

const _maxVisibleRangePageLoads = 2;
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
  final queryProjections = LibraryQueryProjections();
  late final _queryRefresh = LibraryQueryRefreshCoordinator(
    _publications,
    queryProjections,
  );
  late final _querySnapshots = LibraryQuerySnapshotReader(_catalog);
  late final _rootRemovalPages = LibraryRootRemovalPageReader(_catalog);
  late final _timeSnapshots = LibraryTimeSnapshotReader(_catalog);

  late final _timeNavigation = LibraryTimeNavigationRequests(
    cannotPublish: () => _cannotPublish,
    isBlocked: () => _isTimeNavigationBlocked,
    isCompatible: _isCompatibleTimeNavigation,
    load: _loadTimeNavigation,
    onVisibleRangeLoading: _setVisibleRangeLoading,
    onTimeAnchorLoadingReleased: _releaseTimeNavigationLoading,
  );
  int _queryGeneration = 0;
  int _publicationGeneration = 0;
  _VisibleRangeRequest? _pendingVisibleRange;
  _VisibleRangeRequest? _activeVisibleRange;
  bool _isEnsuringVisibleRange = false;
  bool _isVisibleRangeDrainScheduled = false;
  final _retainedWindow = LibraryRetainedCatalogWindow();
  final _queryTransition = LibraryQueryTransition();
  bool _isDisposed = false;

  LibraryState get _state => _readState();

  set _state(LibraryState value) => _publishState(value);

  bool get _cannotPublish => _isDisposed || _isHostDisposed();

  void seed(LibraryState state) {
    _retainedWindow.reset(
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
    _queryTransition.retire();
    _queryRefresh.dispose();
    queryProjections.dispose();
    _publications.dispose();
    _timeNavigation.dispose();
    _pendingVisibleRange = null;
    _activeVisibleRange = null;
  }

  void supersedeExternalRequests() {
    _queryRefresh.invalidate();
    final hadQueryTransition = _queryTransition.retire();
    _publicationGeneration += 1;
    _supersedeQueryWindowRequests();
    if (!_cannotPublish && (_state.isRefreshingQuery || hadQueryTransition)) {
      _state = _state.copyWith(
        queryActivity: _state.isRefreshingQuery
            ? const LibraryQueryIdle()
            : _state.queryActivity,
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

  Future<bool> refreshCurrentQuery() async {
    try {
      return await _queryRefresh.refreshCommitted(
        _refreshCommittedCatalogProjection,
      );
    } on Object catch (error) {
      _reportQueryFailure(error, wasSuperseded: false);
      return false;
    }
  }

  Future<bool> refreshPrimaryScanCatalog({
    LibraryRootConsolidation? rootConsolidation,
  }) => _queryRefresh.refreshCommitted(
    () => _refreshCommittedCatalogProjection(
      rootConsolidation: rootConsolidation,
    ),
  );

  LibraryQueryIntent prepareDirectorySelection() {
    supersedeExternalRequests();
    return _queryRefresh.captureUserIntent();
  }

  Future<LibraryQueryUpdateOutcome> selectCoveredFolder(
    LibraryScanAlreadyCovered folder,
    LibraryQueryIntent intent,
  ) => _queryRefresh.runForIntent(
    intent,
    () => queryProjections.publish(
      (_) => _updateQueryWithOutcome(
        _state.query.copyWith(
          rootId: folder.rootId,
          folderRelativePath: folder.relativePath,
          includeDescendants: true,
        ),
      ),
      preservePosition: false,
    ),
  );

  Future<LibraryQueryUpdateOutcome> _refreshCommittedCatalogProjection({
    LibraryRootConsolidation? rootConsolidation,
  }) {
    final generation = _publicationGeneration;
    return queryProjections.publish((anchor) {
      if (!_canPublishGeneration(generation)) {
        return Future.value(LibraryQueryUpdateOutcome.superseded);
      }
      final readGeneration = ++_publicationGeneration;
      _supersedeQueryWindowRequests();
      return _readCatalogProjection(
        readGeneration,
        anchor,
        rootConsolidation: rootConsolidation,
      );
    });
  }

  Future<LibraryQueryUpdateOutcome> _refreshCatalogProjection(
    int generation, {
    LibraryTimeNavigationRequest? passiveTimeNavigation,
  }) => queryProjections.publish(
    (anchor) => _readCatalogProjection(
      generation,
      anchor,
      passiveTimeNavigation: passiveTimeNavigation,
    ),
  );

  Future<LibraryQueryUpdateOutcome> _readCatalogProjection(
    int generation,
    LibraryQueryAnchor? anchor, {
    LibraryTimeNavigationRequest? passiveTimeNavigation,
    LibraryRootConsolidation? rootConsolidation,
  }) async {
    final query = resolveConsolidatedRootQuery(_state.query, rootConsolidation);
    final projectionRead = queryProjections.beginRead();
    bool canPublish() {
      if (!_canPublishGeneration(generation) ||
          !queryProjections.accepts(projectionRead)) {
        return false;
      }
      if (passiveTimeNavigation == null) {
        return true;
      }
      return _timeNavigation.accepts(passiveTimeNavigation) &&
          !_timeNavigation.ownsExplicitIntent(passiveTimeNavigation);
    }

    final result = await _querySnapshots.loadCurrent(
      query: query,
      anchor: anchor,
      canPublish: canPublish,
    );
    if (result == null || !canPublish()) {
      return LibraryQueryUpdateOutcome.superseded;
    }
    _publishFirstCatalogPage(
      generation: generation,
      snapshot: result.snapshot,
      query: query,
      timeline: result.timeline,
      isLoadingTimeline: false,
    );
    return LibraryQueryUpdateOutcome.applied;
  }

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
    LibraryQueryProjectionRead? projectionRead,
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
    final currentTransition = _queryTransition.currentFor(
      _publicationGeneration,
    );
    if (!forceRefresh &&
        normalized == _state.query &&
        currentTransition == null) {
      return LibraryQueryUpdateOutcome.applied;
    }
    if (!forceRefresh &&
        normalized == _state.query &&
        currentTransition != null) {
      _supersedeQueryWindowRequests();
      _publicationGeneration += 1;
      _queryTransition.finish(currentTransition);
      _state = currentTransition.baseState;
      return LibraryQueryUpdateOutcome.applied;
    }
    final hasVisibleRangeRequest =
        _pendingVisibleRange != null || _activeVisibleRange != null;
    if (!LibraryBrowseAdmission(
      _state,
    ).canStartQuery(hasVisibleRangeRequest: hasVisibleRangeRequest)) {
      return LibraryQueryUpdateOutcome.busy;
    }
    _supersedeQueryWindowRequests();
    final priorGeneration = _publicationGeneration;
    final requestGeneration = ++_publicationGeneration;
    final transition = _queryTransition.begin(
      generation: requestGeneration,
      previousGeneration: priorGeneration,
      currentState: _state,
    );
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
    bool canPublish() {
      return _canPublishGeneration(requestGeneration) &&
          (projectionRead == null || queryProjections.accepts(projectionRead));
    }

    try {
      final result = await _querySnapshots.loadCurrent(
        query: normalized,
        anchor: anchorLocationId == null
            ? null
            : LibraryQueryAnchor(
                requestedLocationId: anchorLocationId,
                assetId: anchorAssetId,
                fallbackGlobalItemIndex: fallbackGlobalItemIndex ?? 0,
              ),
        minimumRevision: minimumCatalogRevision,
        canPublish: canPublish,
      );
      if (result == null || !canPublish()) {
        return LibraryQueryUpdateOutcome.superseded;
      }
      final snapshot = result.snapshot;
      final timeline = result.timeline;
      final baseState = transition.baseState;
      final windowStart =
          snapshot.queryAnchorResolution?.windowStartItemOffset ?? 0;
      _retainedWindow.reset(
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
      _queryTransition.finish(transition);
      return LibraryQueryUpdateOutcome.applied;
    } on Object catch (error) {
      final wasSuperseded = !canPublish();
      _reportQueryFailure(error, wasSuperseded: wasSuperseded);
      if (wasSuperseded) {
        return LibraryQueryUpdateOutcome.superseded;
      }
      final baseState = transition.baseState;
      _queryTransition.finish(transition);
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
    } finally {
      if (_queryTransition.finish(transition) &&
          _canPublishGeneration(requestGeneration)) {
        _state = _state.copyWith(isLoadingTimeline: false);
      }
    }
  }

  void _reportQueryFailure(Object error, {required bool wasSuperseded}) {
    if (!kDebugMode) {
      return;
    }
    final code = error is LibraryCatalogFailure
        ? error.code
        : error.runtimeType.toString();
    debugPrint(
      "[Ame query] outcome=${wasSuperseded ? 'superseded' : 'failed'} code=$code",
    );
  }

  Future<LibraryQueryUpdateOutcome> refreshFromSynchronization({
    required BigInt catalogRevision,
  }) {
    return _queryRefresh.runPassive(() {
      final generation = _publicationGeneration;
      return queryProjections.publish((anchor) {
        if (!_canPublishGeneration(generation)) {
          return Future.value(LibraryQueryUpdateOutcome.superseded);
        }
        return _updateQueryWithOutcome(
          _state.query,
          anchorLocationId: anchor?.requestedLocationId,
          anchorAssetId: anchor?.assetId,
          fallbackGlobalItemIndex: anchor?.fallbackGlobalItemIndex,
          forceRefresh: true,
          minimumCatalogRevision: catalogRevision,
          showRefreshingStatus: false,
          projectionRead: queryProjections.beginRead(),
        );
      });
    });
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
    if (cursor == null || !LibraryBrowseAdmission(_state).canReadPage) {
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
      final window = _retainedWindow.merge(
        snapshot,
        current: _state,
        direction: LibraryPageDirection.next,
      );
      _state = _state.copyWith(
        roots: snapshot.roots,
        assets: window.assets,
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
        windowStartItemOffset: window.startItemOffset,
        previousCursor: window.previousCursor,
        nextCursor: window.nextCursor,
        isLoadingPage: false,
      );
      return window.addedItemCount > 0;
    } on LibraryCatalogFailure catch (error) {
      if (!_canPublishGeneration(generation)) {
        return false;
      }
      if (error.code == "catalog_cursor_stale") {
        try {
          await _queryRefresh.refreshCommitted(
            () => _refreshCatalogProjection(generation),
          );
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
    if (cursor == null || !LibraryBrowseAdmission(_state).canReadPage) {
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
      final window = _retainedWindow.merge(
        snapshot,
        current: _state,
        direction: LibraryPageDirection.previous,
      );
      _state = _state.copyWith(
        roots: snapshot.roots,
        assets: window.assets,
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
        windowStartItemOffset: window.startItemOffset,
        previousCursor: window.previousCursor,
        nextCursor: window.nextCursor,
        isLoadingPreviousPage: false,
      );
      return snapshot.assets.isNotEmpty;
    } on LibraryCatalogFailure catch (error) {
      if (!_canPublishGeneration(generation)) {
        return false;
      }
      if (error.code == "catalog_cursor_stale") {
        try {
          await _queryRefresh.refreshCommitted(
            () => _refreshCatalogProjection(generation),
          );
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
    return _timeNavigation.request(
      query: _state.query,
      timeline: timeline,
      anchor: anchor,
      globalItemOffset: globalItemOffset,
      ownsVisibleRange: ownsVisibleRange,
      needsVisibleRangeLoading: needsVisibleRangeLoading,
    );
  }

  bool get _isTimeNavigationBlocked =>
      !LibraryBrowseAdmission(_state).canNavigateTime;

  bool _isCompatibleTimeNavigation(LibraryTimeNavigationRequest request) {
    final timeline = _state.timeline;
    return timeline != null &&
        timeline.revision == request.currentTimeline.revision &&
        timeline.queryId == request.currentTimeline.queryId &&
        _state.query == request.query;
  }

  Future<bool> _loadTimeNavigation(LibraryTimeNavigationRequest request) async {
    final hadQueryTransition = _queryTransition.retire();
    final generation = ++_publicationGeneration;
    _timeNavigation.beginLoading(request);
    _state = _state.copyWith(
      isLoadingTimeAnchor: true,
      isLoadingTimeline: hadQueryTransition ? false : _state.isLoadingTimeline,
      timeNavigationErrorMessage: null,
    );
    try {
      final result = await _timeSnapshots.load(
        request,
        canPublish: () => _canPublishTimeNavigation(request, generation),
        ownsExplicitIntent: () => _timeNavigation.ownsExplicitIntent(request),
        recoverPassive: () async {
          await _queryRefresh.refreshCommitted(
            () => _refreshCatalogProjection(
              generation,
              passiveTimeNavigation: request,
            ),
          );
        },
      );
      if (result == null || !_canPublishTimeNavigation(request, generation)) {
        return false;
      }
      final snapshot = result.snapshot;
      _retainPreviewPending(snapshot.assets.map((asset) => asset.locationId));
      _retainedWindow.reset(
        assets: snapshot.assets,
        startItemOffset: result.windowStartItemOffset,
        previousCursor: snapshot.previousCursor,
        nextCursor: snapshot.nextCursor,
      );
      _timeNavigation.adoptResolution(request, result);
      _state = publishLibraryTimeSnapshot(_state, result);
      return true;
    } on LibraryCatalogFailure catch (error) {
      if (!_canPublishTimeNavigation(request, generation)) {
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
      _timeNavigation.releaseLoading(request);
    }
  }

  bool _canPublishTimeNavigation(
    LibraryTimeNavigationRequest request,
    int generation,
  ) {
    return _canPublishGeneration(generation) &&
        _timeNavigation.accepts(request);
  }

  void _releaseTimeNavigationLoading() {
    if (!_cannotPublish && _state.isLoadingTimeAnchor) {
      _state = _state.copyWith(isLoadingTimeAnchor: false);
    }
  }

  void cancelTimeNavigation() {
    if (_cannotPublish) {
      return;
    }
    if (!_timeNavigation.cancel()) {
      if (_state.activeTimeAnchor != null) {
        _state = _state.copyWith(activeTimeAnchor: null);
      }
      return;
    }
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
        _queryTransition.currentFor(_publicationGeneration) != null ||
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
        !_timeNavigation.retainsVisibleRange(
          start: request.start,
          end: request.end,
        )) {
      return;
    }
    if (!_timeNavigation.hasVisibleRangeOwner) {
      _timeNavigation.retainPassiveInRange(
        start: request.start,
        end: request.end,
      );
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
        _timeNavigation.hasVisibleRangeLoading;
    if (_cannotPublish || _state.isLoadingVisibleRange == shouldShowLoading) {
      return;
    }
    _state = _state.copyWith(isLoadingVisibleRange: shouldShowLoading);
  }

  Future<void> _loadVisibleRange(_VisibleRangeRequest request) async {
    for (var attempt = 0; attempt < _maxVisibleRangePageLoads; attempt++) {
      if (_cannotPublish ||
          _state.assets.isEmpty ||
          !_ownsVisibleRangeRequest(request) ||
          !_timeNavigation.retainsVisibleRange(
            start: request.start,
            end: request.end,
          )) {
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
    _retainedWindow.reset(
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
    LibraryQueryAnchor? anchor,
  }) async {
    final generation = _publicationGeneration;
    final projectionRead = queryProjections.beginRead();
    final query = _state.query;
    final snapshot = await _rootRemovalPages.loadCurrent(
      removedRootId: removedRootId,
      query: query,
      anchor: anchor,
      canPublish: () =>
          _canPublishGeneration(generation) &&
          queryProjections.accepts(projectionRead),
    );
    if (snapshot == null ||
        !_canPublishGeneration(generation) ||
        !queryProjections.accepts(projectionRead)) {
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

  Future<bool> reloadFirstCatalogPage({
    LibraryQueryAnchor? anchor,
    LibraryQueryProjectionRead? projectionRead,
  }) async {
    final generation = _publicationGeneration;
    final query = _state.query;
    final result = await _querySnapshots.load(query: query, anchor: anchor);
    final snapshot = result.snapshot;
    final timeline = result.timeline;
    if (!_canPublishGeneration(generation) ||
        (projectionRead != null && !queryProjections.accepts(projectionRead))) {
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
    if (_cannotPublish ||
        _state.roots.isEmpty ||
        _state.timeline != null ||
        !LibraryBrowseAdmission(_state).canBrowse) {
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
    final windowStart =
        snapshot.queryAnchorResolution?.windowStartItemOffset ?? 0;
    _retainedWindow.reset(
      assets: snapshot.assets,
      startItemOffset: windowStart,
      previousCursor: snapshot.previousCursor,
      nextCursor: snapshot.nextCursor,
    );
    _state = LibraryState.fromSnapshot(snapshot, query: query).copyWith(
      windowStartItemOffset: windowStart,
      queryActivity: previousState.queryActivity,
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
    _timeNavigation.supersedeQuery();

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
