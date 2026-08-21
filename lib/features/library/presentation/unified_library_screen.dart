import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "../../../app/notifications/ame_notification_strings.dart";
import "../../../app/presentation/ame_notifications.dart";
import "../adapters/windows_library_platform_actions.dart";
import "../../settings/application/ame_preferences.dart";
import "../../settings/presentation/ame_settings_page.dart";
import "../application/library_catalog.dart";
import "../application/library_controller.dart";
import "../application/library_folder_controller.dart";
import "../application/library_layout_manifest_catalog.dart";
import "../application/library_synchronization.dart";
import "../application/library_view_preferences.dart";
import "../domain/gallery_layout_manifest.dart";
import "../domain/library_folder_models.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "../domain/library_synchronization_models.dart";
import "gallery_selection.dart";
import "library_strings.dart";
import "widgets/library_asset_information_sheet.dart";
import "widgets/library_gallery_header.dart";
import "widgets/library_gallery_layout.dart";
import "widgets/library_gallery_states.dart";
import "widgets/library_gallery_wall.dart";
import "widgets/library_global_bar.dart";
import "widgets/library_image_viewer.dart";
import "widgets/library_main_surface.dart";
import "widgets/library_navigation.dart";
import "widgets/library_navigation_resize_handle.dart";
import "widgets/library_task_surface.dart";
import "widgets/library_time_navigation.dart";
import "widgets/library_virtual_gallery_geometry.dart";

class UnifiedLibraryScreen extends ConsumerStatefulWidget {
  const UnifiedLibraryScreen({super.key});

  @override
  ConsumerState<UnifiedLibraryScreen> createState() =>
      _UnifiedLibraryScreenState();
}

class _UnifiedLibraryScreenState extends ConsumerState<UnifiedLibraryScreen> {
  static const _layoutDimensionQuietDelay = Duration(milliseconds: 160);
  static const _layoutDimensionMaximumDelay = Duration(milliseconds: 600);
  static const _retrySynchronizationNotificationAction =
      "library.retrySynchronization";
  static const _synchronizationRefreshNotificationKey =
      "library.synchronizationRefresh";

  late final ScrollController _galleryScrollController;
  final Set<ScrollPosition> _galleryScrollPositions = {};
  late final LibraryController _libraryController;
  late final AmeNotificationController _notificationController;
  late final TextEditingController _searchController;
  late GallerySelection _selection;
  late String _selectionStableQueryId;
  late GalleryLayoutShape _layoutShape;
  late GalleryThumbnailSize _thumbnailSize;
  late double _sidebarWidth;
  bool _isSidebarResizing = false;
  String? _viewerAssetId;
  String? _viewerLocationId;
  LibraryAsset? _retainedViewerAsset;
  int _timelineSemanticsGeneration = 0;
  bool _isSelecting = false;
  bool _isNavigatingViewer = false;
  bool _isRestoringPreviousWindow = false;
  _LibraryDestination _destination = _LibraryDestination.gallery;
  late final ValueNotifier<_LibraryGalleryLayoutSnapshot?>
  _galleryLayoutSnapshot;
  LibraryGalleryVisiblePosition? _visibleGalleryPosition;
  LibraryGalleryLayoutTransition? _galleryLayoutTransition;
  LibraryGalleryVisiblePosition? _pendingQueryPosition;
  final LibraryGalleryPositionResolver _galleryPositionResolver =
      LibraryGalleryPositionResolver();
  int _queryTransitionGeneration = 0;
  int _galleryLayoutTransitionGeneration = 0;
  Timer? _searchDebounce;
  Timer? _layoutDimensionSettleTimer;
  Timer? _layoutDimensionDeadlineTimer;
  StreamSubscription<LibraryGalleryLayoutDimensionUpdate>?
  _layoutDimensionSubscription;
  StreamSubscription<LibrarySynchronizationSnapshot>?
  _synchronizationSubscription;
  late LibrarySynchronizationSnapshot _synchronizationSnapshot;
  BigInt? _pendingSynchronizationRevision;
  bool _isApplyingSynchronizationRevision = false;
  bool _hasSynchronizationRefreshFailure = false;
  final Map<String, String> _synchronizationNotificationKeys = {};
  Timer? _synchronizationRefreshRetry;
  final Map<int, LibraryGalleryLayoutDimensionUpdate>
  _pendingLayoutDimensionUpdates = {};
  final Map<int, LibraryGalleryLayoutDimensionUpdate>
  _deferredLayoutDimensionUpdates = {};
  final Map<int, LibraryGalleryLayoutDimensionUpdate>
  _publishedLayoutDimensionUpdates = {};
  LibraryGalleryLayoutManifest? _dimensionUpdateBaseManifest;
  LibraryGalleryLayoutManifest? _dimensionUpdatedManifest;
  BigInt? _layoutDimensionRevision;
  String _layoutDimensionQueryId = "";
  LibraryGalleryVisibleRange? _visibleGalleryRange;
  LibraryGalleryVisibleRange? _layoutDimensionRecoveryRange;
  LibraryGalleryVisiblePosition? _layoutDimensionRecoveryAnchor;
  bool _isGalleryUserScrolling = false;
  bool _isDisposing = false;
  int _dimensionUpdateGeneration = 0;
  int _dimensionUpdatedManifestGeneration = -1;

  @override
  void initState() {
    super.initState();
    _galleryScrollController = ScrollController(
      onAttach: _handleGalleryScrollPositionAttached,
      onDetach: _handleGalleryScrollPositionDetached,
    );
    _libraryController = ref.read(libraryControllerProvider.notifier);
    _notificationController = ref.read(
      ameNotificationControllerProvider.notifier,
    );
    final state = ref.read(libraryControllerProvider);
    final viewPreferences = ref.read(initialLibraryViewPreferencesProvider);
    final amePreferences = ref.read(initialAmePreferencesProvider);
    _searchController = TextEditingController(text: state.query.searchText);
    _galleryLayoutSnapshot = ValueNotifier(null);
    _selection = GallerySelection.empty(_queryId(state));
    _selectionStableQueryId = _stableQueryId(state);
    _layoutShape = viewPreferences.layoutShape;
    _thumbnailSize = viewPreferences.thumbnailSize;
    _sidebarWidth = amePreferences.sidebarWidth
        .clamp(ameMinimumSidebarWidth, ameMaximumSidebarWidth)
        .toDouble();
    _layoutDimensionSubscription = _libraryController
        .watchLayoutDimensionUpdates()
        .listen(_handleLayoutDimensionUpdate);
    final synchronization = ref.read(librarySynchronizationProvider);
    _synchronizationSubscription = synchronization.watch().listen(
      _handleSynchronizationSnapshot,
    );
    _synchronizationSnapshot = synchronization.current;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _recordSynchronizationNotifications(_synchronizationSnapshot);
      }
    });
    _queueSynchronizationRevision(_synchronizationSnapshot);
  }

  @override
  void dispose() {
    _isDisposing = true;
    if (_viewerAssetId != null) {
      _libraryController.updateViewerPreviewDemand(null);
    }
    _searchDebounce?.cancel();
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionDeadlineTimer?.cancel();
    _synchronizationRefreshRetry?.cancel();
    unawaited(_layoutDimensionSubscription?.cancel());
    unawaited(_synchronizationSubscription?.cancel());
    for (final position in _galleryScrollPositions) {
      position.isScrollingNotifier.removeListener(
        _synchronizeGalleryScrollActivity,
      );
    }
    _galleryScrollPositions.clear();
    _galleryScrollController.dispose();
    _galleryLayoutSnapshot.dispose();
    _searchController.dispose();
    super.dispose();
  }

  void _handleSynchronizationSnapshot(LibrarySynchronizationSnapshot snapshot) {
    if (!mounted) {
      return;
    }
    _recordSynchronizationNotifications(snapshot);
    setState(() => _synchronizationSnapshot = snapshot);
    _queueSynchronizationRevision(snapshot);
  }

  void _queueSynchronizationRevision(LibrarySynchronizationSnapshot snapshot) {
    final currentRevision = ref.read(libraryControllerProvider).catalogRevision;
    if (snapshot.catalogRevision <= (currentRevision ?? BigInt.zero)) {
      return;
    }
    _retainPendingSynchronizationRevision(snapshot.catalogRevision);
    _scheduleSynchronizationRefresh();
  }

  void _retainPendingSynchronizationRevision(BigInt revision) {
    final pendingRevision = _pendingSynchronizationRevision;
    if (pendingRevision == null || revision > pendingRevision) {
      _pendingSynchronizationRevision = revision;
    }
  }

  void _scheduleSynchronizationRefresh() {
    if (_isApplyingSynchronizationRevision || !mounted) {
      return;
    }
    _synchronizationRefreshRetry?.cancel();
    _synchronizationRefreshRetry = Timer(
      Duration.zero,
      () => unawaited(_drainSynchronizationRefresh()),
    );
  }

  Future<void> _drainSynchronizationRefresh() async {
    if (_isApplyingSynchronizationRevision || !mounted) {
      return;
    }
    final targetRevision = _pendingSynchronizationRevision;
    if (targetRevision == null) {
      return;
    }
    final state = ref.read(libraryControllerProvider);
    if (state.isBusy || state.isLoadingTimeAnchor) {
      _synchronizationRefreshRetry = Timer(
        const Duration(milliseconds: 250),
        _scheduleSynchronizationRefresh,
      );
      return;
    }
    _isApplyingSynchronizationRevision = true;
    _pendingSynchronizationRevision = null;
    late final LibraryQueryUpdateOutcome updateOutcome;
    try {
      updateOutcome = await _applyQuery(
        state.query,
        synchronizationRevision: targetRevision,
      );
    } finally {
      _isApplyingSynchronizationRevision = false;
    }
    if (!mounted) {
      return;
    }
    final revisionQueuedDuringAttempt = _pendingSynchronizationRevision;
    switch (updateOutcome) {
      case LibraryQueryUpdateOutcome.applied:
        _setSynchronizationRefreshFailure(false);
        break;
      case LibraryQueryUpdateOutcome.busy:
      case LibraryQueryUpdateOutcome.superseded:
        _retainPendingSynchronizationRevision(targetRevision);
        _synchronizationRefreshRetry = Timer(
          const Duration(milliseconds: 250),
          _scheduleSynchronizationRefresh,
        );
        return;
      case LibraryQueryUpdateOutcome.failed:
        _retainPendingSynchronizationRevision(targetRevision);
        _setSynchronizationRefreshFailure(true);
        if (revisionQueuedDuringAttempt != null) {
          _scheduleSynchronizationRefresh();
        }
        return;
    }
    final publishedRevision =
        ref.read(libraryControllerProvider).catalogRevision ?? BigInt.zero;
    if (publishedRevision < targetRevision) {
      _retainPendingSynchronizationRevision(targetRevision);
      _setSynchronizationRefreshFailure(true);
      if (revisionQueuedDuringAttempt != null) {
        _scheduleSynchronizationRefresh();
      }
      return;
    }
    if (_pendingSynchronizationRevision != null) {
      _scheduleSynchronizationRefresh();
    }
  }

  void _setSynchronizationRefreshFailure(bool hasFailure) {
    if (!mounted || _hasSynchronizationRefreshFailure == hasFailure) {
      return;
    }
    if (hasFailure) {
      _notificationController.publish(
        const AmeNotificationDraft(
          title: LibraryStrings.synchronizationRefreshFailureTitle,
          message: LibraryStrings.synchronizationRefreshFailureMessage,
          severity: AmeNotificationSeverity.error,
          dedupeKey: _synchronizationRefreshNotificationKey,
          actionId: _retrySynchronizationNotificationAction,
          actionLabel: LibraryStrings.retry,
          isPersistent: true,
        ),
      );
    } else {
      _notificationController.resolve(_synchronizationRefreshNotificationKey);
    }
    setState(() => _hasSynchronizationRefreshFailure = hasFailure);
  }

  Future<void> _retrySynchronizationRefresh() {
    if (_pendingSynchronizationRevision != null) {
      _scheduleSynchronizationRefresh();
    }
    return Future<void>.value();
  }

  void _recordSynchronizationNotifications(
    LibrarySynchronizationSnapshot snapshot,
  ) {
    final roots = {
      for (final root in ref.read(libraryControllerProvider).roots)
        root.id: root,
    };
    final reportedRootIds = snapshot.roots.keys.toSet();
    for (final rootId in _synchronizationNotificationKeys.keys.toList()) {
      if (!reportedRootIds.contains(rootId)) {
        final key = _synchronizationNotificationKeys.remove(rootId);
        if (key != null) {
          _notificationController.resolve(key);
        }
      }
    }
    for (final entry in snapshot.roots.entries) {
      final rootId = entry.key;
      final status = entry.value;
      final root = roots[rootId];
      final priorKey = _synchronizationNotificationKeys[rootId];
      switch (status.freshness) {
        case LibraryCatalogFreshness.needsReconciliation:
          final key = _synchronizationNotificationKey(status);
          if (priorKey != null && priorKey != key) {
            _notificationController.resolve(priorKey);
          }
          _synchronizationNotificationKeys[rootId] = key;
          _notificationController.publish(
            AmeNotificationDraft(
              title: LibraryStrings.synchronizationReconciliationTitle(
                _notificationRootName(root, rootId),
              ),
              message: _synchronizationNotificationMessage(status),
              severity: _synchronizationNotificationSeverity(status),
              dedupeKey: key,
              detail: _synchronizationNotificationDetail(status),
              sourcePath: root?.displayPath,
              technicalCode: status.lastIssueCode,
              isPersistent: true,
            ),
          );
          break;
        case LibraryCatalogFreshness.synchronized:
          if (priorKey != null) {
            _notificationController.resolve(priorKey);
          }
          _synchronizationNotificationKeys.remove(rootId);
          break;
        case LibraryCatalogFreshness.unavailable:
          if (priorKey != null) {
            _notificationController.resolve(priorKey);
            _synchronizationNotificationKeys.remove(rootId);
          }
          break;
        case LibraryCatalogFreshness.updating:
          break;
      }
    }
  }

  String _synchronizationNotificationKey(
    LibraryRootSynchronizationStatus status,
  ) {
    return "library.rootReconciliation:${status.rootId}";
  }

  String _synchronizationNotificationMessage(
    LibraryRootSynchronizationStatus status,
  ) {
    final issueMessage = _synchronizationIssueMessage(status.lastIssueCode);
    if (issueMessage != null) {
      return issueMessage;
    }
    return switch (status.freshnessCause) {
      LibraryCatalogFreshnessCause.changeSourceUnhealthy =>
        LibraryStrings.synchronizationSourceUnhealthy,
      LibraryCatalogFreshnessCause.evidenceGap =>
        LibraryStrings.synchronizationEvidenceGap,
      LibraryCatalogFreshnessCause.boundedCapacityExceeded =>
        LibraryStrings.synchronizationCapacityExceeded,
      LibraryCatalogFreshnessCause.noPendingChanges ||
      LibraryCatalogFreshnessCause.pendingChanges ||
      LibraryCatalogFreshnessCause.rootUnavailable =>
        LibraryStrings.synchronizationNeedsReconciliation,
    };
  }

  String? _synchronizationIssueMessage(String? issueCode) {
    final message = switch (issueCode) {
      "change_source_callback_access_denied" ||
      "change_source_start_access_denied" ||
      "change_source_watch_access_denied" =>
        LibraryStrings.synchronizationMonitoringAccessDenied,
      "change_source_callback_path_unavailable" ||
      "change_source_root_removed" ||
      "change_source_root_unavailable" =>
        LibraryStrings.synchronizationMonitoringPathUnavailable,
      "change_source_callback_capacity_exceeded" ||
      "change_source_ingress_overflow" ||
      "change_source_rescan_required" ||
      "change_source_start_capacity_exceeded" ||
      "change_source_watch_capacity_exceeded" =>
        LibraryStrings.synchronizationMonitoringCapacityExceeded,
      "change_source_event_incomplete" || "change_source_rename_incomplete" =>
        LibraryStrings.synchronizationMonitoringEventIncomplete,
      "change_source_callback_failed" ||
      "change_source_callback_invalid_configuration" ||
      "change_source_callback_io_failed" ||
      "change_source_callback_watch_missing" ||
      "change_source_ingress_disconnected" ||
      "change_source_state_unavailable" ||
      "change_source_start_failed" ||
      "change_source_start_invalid_configuration" ||
      "change_source_watch_invalid_configuration" ||
      "change_source_watch_failed" =>
        LibraryStrings.synchronizationMonitoringFailed,
      _ => null,
    };
    if (message != null || issueCode == null) {
      return message;
    }
    if (issueCode.startsWith("authoritative_") ||
        issueCode.startsWith("library_reconciliation_")) {
      return LibraryStrings.synchronizationRecoveryFailed;
    }
    if (issueCode.startsWith("catalog_") ||
        issueCode.startsWith("change_queue_") ||
        issueCode.startsWith("database_")) {
      return LibraryStrings.synchronizationPersistenceFailed;
    }
    return null;
  }

  String? _synchronizationNotificationDetail(
    LibraryRootSynchronizationStatus status,
  ) {
    final details = <String>[];
    if (status.pendingChangeCount > BigInt.zero) {
      details.add("${status.pendingChangeCount} 项等待处理");
    }
    if (status.retryWaitCount > BigInt.zero) {
      details.add("${status.retryWaitCount} 项等待重试");
    }
    if (status.freshnessUnknownCount > BigInt.zero) {
      details.add("${status.freshnessUnknownCount} 项状态尚未确认");
    }
    return details.isEmpty ? null : details.join(" · ");
  }

  AmeNotificationSeverity _synchronizationNotificationSeverity(
    LibraryRootSynchronizationStatus status,
  ) {
    final issueCode = status.lastIssueCode;
    if (status.sourceStatus == LibraryChangeSourceStatus.failed ||
        issueCode?.startsWith("catalog_") == true ||
        issueCode?.startsWith("change_queue_") == true ||
        issueCode?.startsWith("database_") == true) {
      return AmeNotificationSeverity.error;
    }
    return AmeNotificationSeverity.warning;
  }

  String _notificationRootName(LibraryRoot? root, String rootId) {
    final displayPath = root?.displayPath.trim();
    if (displayPath == null || displayPath.isEmpty) {
      return rootId;
    }
    final normalized = displayPath.replaceAll("\\", "/");
    final segments = normalized
        .split("/")
        .where((segment) => segment.isNotEmpty)
        .toList();
    return segments.isEmpty ? displayPath : segments.last;
  }

  void _handleNotificationAction(AmeNotificationEntry notification) {
    switch (notification.actionId) {
      case _retrySynchronizationNotificationAction:
        unawaited(_retrySynchronizationRefresh());
        break;
      case null:
        break;
      default:
        break;
    }
  }

  void _showNotificationDetails(AmeNotificationEntry notification) {
    unawaited(
      showAmeNotificationDetails(
        context,
        notification,
        onAction: _handleNotificationAction,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(libraryControllerProvider);
    final notifications = ref.watch(ameNotificationControllerProvider);
    _synchronizeLayoutDimensionContext(state.catalogRevision, state.queryId);
    final amePreferences = ref.watch(amePreferencesControllerProvider);
    final controller = _libraryController;
    final hasLibraryTaskSurface = _showsTaskSurface(state);
    final currentNotification = notifications.current;
    final queryId = _queryId(state);
    final catalogRevision = state.catalogRevision;
    final manifestRequest =
        _layoutShape == GalleryLayoutShape.equalHeight &&
            catalogRevision != null &&
            state.queryId.isNotEmpty
        ? LibraryGalleryLayoutManifestRequest(
            query: state.query,
            revision: catalogRevision,
            queryId: state.queryId,
          )
        : null;
    final baseLayoutManifest = manifestRequest == null
        ? null
        : ref
              .watch(libraryGalleryLayoutManifestProvider(manifestRequest))
              .value;
    final layoutManifest = baseLayoutManifest == null
        ? null
        : _manifestWithRecoveredDimensions(baseLayoutManifest);
    if (_selection.queryId != queryId) {
      final stableQueryId = _stableQueryId(state);
      if (_selectionStableQueryId == stableQueryId) {
        _selection = _selection.rebind(queryId);
      } else {
        _selection = GallerySelection.empty(queryId);
      }
      _selectionStableQueryId = stableQueryId;
      _isSelecting = !_selection.isEmpty;
    }
    final loadedViewerAsset = _viewerAssetId == null
        ? null
        : _assetByStableIdentity(
            state.assets,
            _viewerAssetId!,
            _viewerLocationId,
          );
    final viewerCatalogAsset =
        loadedViewerAsset ??
        (_retainedViewerAsset?.assetId == _viewerAssetId
            ? _retainedViewerAsset
            : null);
    final viewerIndex = viewerCatalogAsset == null
        ? -1
        : state.assets.indexWhere(
            (asset) => asset.locationId == viewerCatalogAsset.locationId,
          );
    if (loadedViewerAsset != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted || _viewerAssetId != loadedViewerAsset.assetId) {
          return;
        }
        _viewerLocationId = loadedViewerAsset.locationId;
        _retainedViewerAsset = loadedViewerAsset;
        controller.updateViewerPreviewDemand(loadedViewerAsset);
      });
    }

    return CallbackShortcuts(
      bindings: viewerCatalogAsset == null
          ? {
              const SingleActivator(
                LogicalKeyboardKey.keyA,
                control: true,
              ): () =>
                  _selectAll(state),
              const SingleActivator(LogicalKeyboardKey.escape):
                  _cancelCurrentMode,
              const SingleActivator(LogicalKeyboardKey.keyD, control: true):
                  _clearSelection,
            }
          : {
              const SingleActivator(LogicalKeyboardKey.escape):
                  _cancelCurrentMode,
            },
      child: Focus(
        autofocus: true,
        child: Scaffold(
          backgroundColor: Theme.of(context).colorScheme.surfaceContainerLow,
          body: Stack(
            children: [
              IndexedStack(
                index: viewerCatalogAsset == null ? 0 : 1,
                children: [
                  Column(
                    children: [
                      LibraryGlobalBar(
                        isBusy: state.isBusy,
                        searchController: _searchController,
                        onSearchChanged: _onSearchChanged,
                        notifications: notifications,
                        onNotificationsOpened:
                            _notificationController.markAllRead,
                        onNotificationSelected: _showNotificationDetails,
                      ),
                      Expanded(
                        child: _buildLibrary(
                          context,
                          state,
                          controller,
                          amePreferences,
                          layoutManifest,
                        ),
                      ),
                    ],
                  ),
                  if (viewerCatalogAsset == null)
                    const SizedBox.shrink()
                  else
                    StreamBuilder<void>(
                      stream: controller.watchPreview(
                        viewerCatalogAsset.locationId,
                      ),
                      builder: (context, snapshot) {
                        final viewerAsset = controller.resolvePreview(
                          viewerCatalogAsset,
                        );
                        return LibraryImageViewer(
                          asset: viewerAsset,
                          wheelBehavior: amePreferences.viewerWheelBehavior,
                          openBehavior: amePreferences.viewerOpenBehavior,
                          position: viewerIndex < 0
                              ? null
                              : state.windowStartItemOffset + viewerIndex + 1,
                          totalItems: _totalItems(state),
                          onPrevious:
                              viewerIndex > 0 ||
                                  (state.hasPreviousAssets &&
                                      !_isNavigatingViewer)
                              ? () => unawaited(_openAdjacentAsset(-1))
                              : null,
                          onNext:
                              viewerIndex >= 0 &&
                                  (viewerIndex < state.assets.length - 1 ||
                                      (state.hasMoreAssets &&
                                          !_isNavigatingViewer))
                              ? () => unawaited(_openAdjacentAsset(1))
                              : null,
                          onBack: _closeViewer,
                          onInformation: () =>
                              _showAssetInformation(viewerAsset),
                          onCopyPath: () => _copyAssetPath(viewerAsset),
                          onRevealFile: () => _revealAsset(viewerAsset),
                        );
                      },
                    ),
                ],
              ),
              if (viewerCatalogAsset == null && hasLibraryTaskSurface)
                Align(
                  alignment: Alignment.bottomCenter,
                  child: Padding(
                    padding: const EdgeInsets.only(bottom: 24),
                    child: LibraryTaskSurface(
                      state: state,
                      onPause: controller.pauseScan,
                      onCancel: controller.cancelScan,
                      onResume: controller.resumePausedScan,
                      onRetry: controller.retry,
                      onDismiss: controller.dismissTaskFeedback,
                    ),
                  ),
                ),
              if (viewerCatalogAsset == null &&
                  !hasLibraryTaskSurface &&
                  currentNotification != null)
                Align(
                  alignment: Alignment.bottomCenter,
                  child: Padding(
                    padding: const EdgeInsets.only(bottom: 24),
                    child: AmeNotificationSurface(
                      key: ValueKey(currentNotification.id),
                      notification: currentNotification,
                      onAction: _handleNotificationAction,
                      onDismiss: _notificationController.dismiss,
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }

  void _handleLayoutDimensionUpdate(
    LibraryGalleryLayoutDimensionUpdate update,
  ) {
    if (!mounted) {
      return;
    }
    final current = ref.read(libraryControllerProvider);
    if (current.catalogRevision != update.revision ||
        current.queryId != update.queryId) {
      return;
    }
    _synchronizeLayoutDimensionContext(update.revision, update.queryId);
    if (!_isGalleryUserScrolling) {
      _ensureLayoutDimensionRecoveryEpoch(current);
    }
    final eligibleRange = _layoutDimensionRecoveryRange ?? _visibleGalleryRange;
    if (eligibleRange?.contains(
          queryId: update.queryId,
          revision: update.revision,
          globalItemIndex: update.globalItemIndex,
        ) ??
        false) {
      _pendingLayoutDimensionUpdates[update.globalItemIndex] = update;
      _scheduleRecoveredDimensionPublication();
    } else {
      _deferredLayoutDimensionUpdates[update.globalItemIndex] = update;
    }
  }

  void _handleVisibleGalleryRangeChanged(LibraryGalleryVisibleRange range) {
    if (!mounted ||
        range.queryId != _layoutDimensionQueryId ||
        range.revision != _layoutDimensionRevision) {
      return;
    }
    final previous = _visibleGalleryRange;
    if (previous != null && previous.matches(range)) {
      return;
    }
    _visibleGalleryRange = range;
    if (_layoutDimensionRecoveryRange != null && !_isGalleryUserScrolling) {
      return;
    }
    if (_isGalleryUserScrolling) {
      _reclassifyLayoutDimensionUpdates(range);
    } else {
      _ensureLayoutDimensionRecoveryEpoch(ref.read(libraryControllerProvider));
    }
    _scheduleRecoveredDimensionPublication();
  }

  void _reclassifyLayoutDimensionUpdates(
    LibraryGalleryVisibleRange eligibleRange,
  ) {
    for (final entry in _pendingLayoutDimensionUpdates.entries.toList()) {
      if (!eligibleRange.containsGlobalItemIndex(entry.key)) {
        _pendingLayoutDimensionUpdates.remove(entry.key);
        _deferredLayoutDimensionUpdates[entry.key] = entry.value;
      }
    }
    for (final entry in _deferredLayoutDimensionUpdates.entries.toList()) {
      if (eligibleRange.containsGlobalItemIndex(entry.key)) {
        _deferredLayoutDimensionUpdates.remove(entry.key);
        _pendingLayoutDimensionUpdates[entry.key] = entry.value;
      }
    }
  }

  void _ensureLayoutDimensionRecoveryEpoch(LibraryState state) {
    if (_layoutDimensionRecoveryRange != null) {
      return;
    }
    final visibleRange = _visibleGalleryRange;
    if (visibleRange == null ||
        visibleRange.queryId != state.queryId ||
        visibleRange.revision != state.catalogRevision) {
      return;
    }
    _layoutDimensionRecoveryRange = visibleRange;
    _layoutDimensionRecoveryAnchor = _freezeCurrentGalleryPosition(state);
    _reclassifyLayoutDimensionUpdates(visibleRange);
  }

  void _handleGalleryUserScrollActivityChanged(bool isScrolling) {
    if (!mounted || _isDisposing || _isGalleryUserScrolling == isScrolling) {
      return;
    }
    _isGalleryUserScrolling = isScrolling;
    if (isScrolling) {
      _layoutDimensionRecoveryRange = null;
      _layoutDimensionRecoveryAnchor = null;
      _layoutDimensionSettleTimer?.cancel();
      _layoutDimensionSettleTimer = null;
      _layoutDimensionDeadlineTimer?.cancel();
      _layoutDimensionDeadlineTimer = null;
      final visibleRange = _visibleGalleryRange;
      if (visibleRange != null) {
        _reclassifyLayoutDimensionUpdates(visibleRange);
      }
      return;
    }
    final current = ref.read(libraryControllerProvider);
    _ensureLayoutDimensionRecoveryEpoch(current);
    _scheduleRecoveredDimensionPublication();
  }

  void _handleGalleryScrollPositionAttached(ScrollPosition position) {
    _galleryScrollPositions.add(position);
    position.isScrollingNotifier.addListener(_synchronizeGalleryScrollActivity);
    _synchronizeGalleryScrollActivity();
  }

  void _handleGalleryScrollPositionDetached(ScrollPosition position) {
    position.isScrollingNotifier.removeListener(
      _synchronizeGalleryScrollActivity,
    );
    _galleryScrollPositions.remove(position);
    _synchronizeGalleryScrollActivity();
  }

  void _synchronizeGalleryScrollActivity() {
    _handleGalleryUserScrollActivityChanged(
      _galleryScrollPositions.any(
        (position) => position.isScrollingNotifier.value,
      ),
    );
  }

  void _scheduleRecoveredDimensionPublication() {
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionSettleTimer = null;
    if (_pendingLayoutDimensionUpdates.isEmpty ||
        _isGalleryUserScrolling ||
        _galleryLayoutTransition != null ||
        _pendingQueryPosition != null) {
      _layoutDimensionDeadlineTimer?.cancel();
      _layoutDimensionDeadlineTimer = null;
      return;
    }
    _layoutDimensionDeadlineTimer ??= Timer(
      _layoutDimensionMaximumDelay,
      _publishRecoveredDimensions,
    );
    _layoutDimensionSettleTimer = Timer(
      _layoutDimensionQuietDelay,
      _publishRecoveredDimensions,
    );
  }

  void _publishRecoveredDimensions() {
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionSettleTimer = null;
    _layoutDimensionDeadlineTimer?.cancel();
    _layoutDimensionDeadlineTimer = null;
    if (!mounted ||
        _pendingLayoutDimensionUpdates.isEmpty ||
        _isGalleryUserScrolling ||
        _galleryLayoutTransition != null ||
        _pendingQueryPosition != null) {
      return;
    }
    final current = ref.read(libraryControllerProvider);
    _ensureLayoutDimensionRecoveryEpoch(current);
    final recoveryAnchor = _layoutDimensionRecoveryAnchor;
    final compatible = <int, LibraryGalleryLayoutDimensionUpdate>{};
    for (final entry in _pendingLayoutDimensionUpdates.entries) {
      final update = entry.value;
      if (current.catalogRevision == update.revision &&
          current.queryId == update.queryId) {
        compatible[entry.key] = update;
      }
    }
    _pendingLayoutDimensionUpdates.clear();
    if (compatible.isEmpty) {
      return;
    }
    setState(() {
      _publishedLayoutDimensionUpdates.addAll(compatible);
      _dimensionUpdateGeneration += 1;
      if (_layoutShape == GalleryLayoutShape.equalHeight &&
          recoveryAnchor != null) {
        _galleryLayoutTransitionGeneration += 1;
        _galleryLayoutTransition = LibraryGalleryLayoutTransition(
          generation: _galleryLayoutTransitionGeneration,
          position: recoveryAnchor,
        );
      }
    });
  }

  LibraryGalleryLayoutManifest _manifestWithRecoveredDimensions(
    LibraryGalleryLayoutManifest manifest,
  ) {
    if (!identical(_dimensionUpdateBaseManifest, manifest) ||
        _dimensionUpdatedManifestGeneration != _dimensionUpdateGeneration) {
      _dimensionUpdateBaseManifest = manifest;
      _dimensionUpdatedManifest = manifest.withDimensionUpdates(
        _publishedLayoutDimensionUpdates.values,
      );
      _dimensionUpdatedManifestGeneration = _dimensionUpdateGeneration;
    }
    return _dimensionUpdatedManifest ?? manifest;
  }

  void _synchronizeLayoutDimensionContext(BigInt? revision, String queryId) {
    if (_layoutDimensionRevision == revision &&
        _layoutDimensionQueryId == queryId) {
      return;
    }
    _layoutDimensionRevision = revision;
    _layoutDimensionQueryId = queryId;
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionSettleTimer = null;
    _layoutDimensionDeadlineTimer?.cancel();
    _layoutDimensionDeadlineTimer = null;
    _pendingLayoutDimensionUpdates.clear();
    _deferredLayoutDimensionUpdates.clear();
    _publishedLayoutDimensionUpdates.clear();
    _dimensionUpdateBaseManifest = null;
    _dimensionUpdatedManifest = null;
    _visibleGalleryRange = null;
    _layoutDimensionRecoveryRange = null;
    _layoutDimensionRecoveryAnchor = null;
    _isGalleryUserScrolling = false;
    _dimensionUpdateGeneration = 0;
    _dimensionUpdatedManifestGeneration = -1;
  }

  Widget _buildLibrary(
    BuildContext context,
    LibraryState state,
    LibraryController controller,
    AmePreferences amePreferences,
    LibraryGalleryLayoutManifest? layoutManifest,
  ) {
    final folderTree = ref.watch(libraryFolderControllerProvider);
    final folderController = ref.read(libraryFolderControllerProvider.notifier);
    return LayoutBuilder(
      builder: (context, constraints) {
        final isCompactNavigation = constraints.maxWidth < 940;
        final navigationWidth = isCompactNavigation ? 76.0 : _sidebarWidth;
        final content = Row(
          children: [
            LibraryNavigation(
              isCompact: isCompactNavigation,
              width: navigationWidth,
              isSettingsSelected: _destination == _LibraryDestination.settings,
              roots: state.roots,
              rootSynchronizationStatuses: _synchronizationSnapshot.roots,
              hasSynchronizationFailure:
                  _synchronizationSnapshot.lastErrorCode != null,
              selectedRootId: state.query.rootId,
              selectedFolderRelativePath: state.query.folderRelativePath,
              transientRootPath: _transientRootPath(state),
              folderTree: folderTree,
              isBusy: state.isBusy,
              onSelectLibrary: () => _selectLibrary(state),
              onSelectRoot: (root) => _selectRoot(state, root),
              onSelectFolder: (root, folder) =>
                  _selectFolder(state, root, folder),
              onExpandFolder: (rootId, parentRelativePath) => _loadFolderBranch(
                folderController,
                state,
                rootId,
                parentRelativePath,
              ),
              onLoadMoreFolders: (rootId, parentRelativePath) =>
                  _loadFolderBranch(
                    folderController,
                    state,
                    rootId,
                    parentRelativePath,
                    loadMore: true,
                  ),
              onAddSource: controller.chooseDirectoryAndScan,
              onOpenSettings: _openSettings,
              onUpdateRoot: controller.scanDirectory,
              onOpenRoot: _openRoot,
              onOpenFolder: _openFolder,
              onRemoveRoot: _confirmRemoveRoot,
            ),
            if (isCompactNavigation) const SizedBox(width: 1),
            Expanded(
              child: LibraryMainSurface(
                child: _destination == _LibraryDestination.settings
                    ? AmeSettingsPage(hasLibraryRoots: state.roots.isNotEmpty)
                    : Column(
                        children: [
                          LibraryGalleryHeader(
                            galleryTitle: _galleryTitle(state),
                            totalItems: _totalItems(state),
                            selectedCount: _selection.selectedCount(
                              _totalItems(state),
                            ),
                            isSelecting: _isSelecting,
                            layoutShape: _layoutShape,
                            thumbnailSize: _thumbnailSize,
                            sortKey: state.query.sortKey,
                            sortDirection: state.query.sortDirection,
                            onBeginSelection: () =>
                                setState(() => _isSelecting = true),
                            onCancelSelection: _clearSelection,
                            onSelectAll: () => _selectAll(state),
                            onLayoutShapeChanged: _changeLayoutShape,
                            onThumbnailSizeChanged: _changeThumbnailSize,
                            onSortKeyChanged: (value) =>
                                _changeSortKey(state, value),
                            onSortDirectionChanged: (value) =>
                                _changeSortDirection(state, value),
                          ),
                          if (state.isLoadingPage ||
                              state.isLoadingPreviousPage ||
                              state.isLoadingTimeAnchor)
                            const LinearProgressIndicator(
                              key: Key("library-top-loading"),
                              minHeight: 2,
                              semanticsLabel: "正在加载图片",
                            ),
                          Expanded(
                            child: _buildGalleryBody(
                              state,
                              controller,
                              layoutManifest,
                            ),
                          ),
                        ],
                      ),
              ),
            ),
          ],
        );
        if (isCompactNavigation) {
          return content;
        }
        return Stack(
          children: [
            content,
            Positioned(
              left:
                  navigationWidth -
                  LibraryNavigationResizeHandle.hitTargetWidth / 2,
              top: 0,
              bottom: 0,
              child: LibraryNavigationResizeHandle(
                width: _sidebarWidth,
                minimumWidth: ameMinimumSidebarWidth,
                maximumWidth: ameMaximumSidebarWidth,
                defaultWidth: ameDefaultSidebarWidth,
                onWidthChangeStart: _beginSidebarResize,
                onWidthChanged: _changeSidebarWidth,
                onWidthChangeEnd: (width) =>
                    _endSidebarResize(amePreferences, width),
                onWidthChangeCancel: _cancelSidebarResize,
              ),
            ),
          ],
        );
      },
    );
  }

  Widget _buildGalleryBody(
    LibraryState state,
    LibraryController controller,
    LibraryGalleryLayoutManifest? layoutManifest,
  ) {
    if (state.assets.isEmpty) {
      if (state.status == LibraryStatus.refreshing || state.isLoadingTimeline) {
        return const GalleryLoadingState();
      }
      if (state.roots.isNotEmpty) {
        return NoGalleryResults(query: state.query);
      }
      return EmptyLibrary(
        state: state,
        onImport: controller.chooseDirectoryAndScan,
      );
    }
    return Row(
      children: [
        Expanded(
          child: LibraryGalleryWall(
            state: state,
            controller: controller,
            scrollController: _galleryScrollController,
            layoutShape: _layoutShape,
            thumbnailSize: _thumbnailSize,
            selection: _selection,
            isSelecting: _isSelecting,
            isSidebarResizing: _isSidebarResizing,
            onOpen: _openAsset,
            onToggleSelection: _toggleSelection,
            onViewInformation: _showAssetInformation,
            onCopyPath: _copyAssetPath,
            onRevealFile: _revealAsset,
            onVisiblePositionChanged: (position) {
              if (_galleryLayoutTransition == null &&
                  _pendingQueryPosition == null) {
                _visibleGalleryPosition = position;
              }
            },
            onVisibleRangeChanged: _handleVisibleGalleryRangeChanged,
            onLoadPrevious: () =>
                _loadPreviousPagePreservingPosition(controller),
            onLayoutChanged: _handleGalleryLayoutChanged,
            layoutManifest: layoutManifest,
            initialQueryWidePosition: _initialQueryWidePosition(state),
            layoutTransition: _galleryLayoutTransition,
            onLayoutTransitionApplied: _completeGalleryLayoutTransition,
            positionResolver: _galleryPositionResolver,
          ),
        ),
        ValueListenableBuilder<_LibraryGalleryLayoutSnapshot?>(
          valueListenable: _galleryLayoutSnapshot,
          builder: (context, snapshot, child) {
            final activeSnapshot = snapshot?.matches(state) ?? false
                ? snapshot
                : null;
            return LibraryTimeNavigation(
              key: ValueKey<int>(_timelineSemanticsGeneration),
              isLoading: state.isLoadingTimeline,
              scrollController: _galleryScrollController,
              layoutMetrics: activeSnapshot?.metrics,
              timeline: state.timeline,
              layoutShape: _layoutShape,
              virtualGeometry: activeSnapshot?.virtualGeometry,
              windowStartItemOffset: state.windowStartItemOffset,
              loadedItemCount: state.assets.length,
              onSeek: (bucket, itemOffset) =>
                  _seekTimeline(controller, bucket, itemOffset),
              onBeginNavigation: _beginTimelineNavigation,
              onCancelSeek: controller.cancelTimeNavigation,
              onPrefetch: (bucket, itemOffset) =>
                  controller.prefetchTime(bucket, itemOffset: itemOffset),
            );
          },
        ),
      ],
    );
  }

  void _openAsset(LibraryAsset asset) {
    ref
        .read(libraryControllerProvider.notifier)
        .updateViewerPreviewDemand(asset);
    setState(() {
      _viewerAssetId = asset.assetId;
      _viewerLocationId = asset.locationId;
      _retainedViewerAsset = asset;
    });
  }

  void _closeViewer() {
    ref
        .read(libraryControllerProvider.notifier)
        .updateViewerPreviewDemand(null);
    setState(() {
      _timelineSemanticsGeneration += 1;
      _viewerAssetId = null;
      _viewerLocationId = null;
      _retainedViewerAsset = null;
    });
  }

  Future<void> _openAdjacentAsset(int direction) async {
    final currentAssetId = _viewerAssetId;
    final currentLocationId = _viewerLocationId;
    if (currentAssetId == null ||
        currentLocationId == null ||
        _isNavigatingViewer) {
      return;
    }
    setState(() => _isNavigatingViewer = true);
    try {
      var state = ref.read(libraryControllerProvider);
      var currentIndex = state.assets.indexWhere(
        (asset) =>
            asset.assetId == currentAssetId &&
            asset.locationId == currentLocationId,
      );
      var targetIndex = currentIndex + direction;
      if (targetIndex < 0 && state.hasPreviousAssets) {
        await _loadPreviousAssetsForViewer(
          ref.read(libraryControllerProvider.notifier),
        );
      } else if (targetIndex >= state.assets.length && state.hasMoreAssets) {
        await ref.read(libraryControllerProvider.notifier).loadNextPage();
      }
      if (!mounted ||
          _viewerAssetId != currentAssetId ||
          _viewerLocationId != currentLocationId) {
        return;
      }
      state = ref.read(libraryControllerProvider);
      currentIndex = state.assets.indexWhere(
        (asset) =>
            asset.assetId == currentAssetId &&
            asset.locationId == currentLocationId,
      );
      targetIndex = currentIndex + direction;
      if (currentIndex < 0 ||
          targetIndex < 0 ||
          targetIndex >= state.assets.length) {
        return;
      }
      final target = state.assets[targetIndex];
      ref
          .read(libraryControllerProvider.notifier)
          .updateViewerPreviewDemand(target);
      setState(() {
        _viewerAssetId = target.assetId;
        _viewerLocationId = target.locationId;
        _retainedViewerAsset = target;
      });
    } finally {
      if (mounted) {
        setState(() => _isNavigatingViewer = false);
      }
    }
  }

  Future<void> _loadPreviousAssetsForViewer(
    LibraryController controller,
  ) async {
    if (_galleryScrollController.hasClients) {
      await _loadPreviousPagePreservingPosition(controller);
      return;
    }
    await controller.loadPreviousPage();
  }

  void _toggleSelection(LibraryAsset asset) {
    final totalItems = _totalItems(ref.read(libraryControllerProvider));
    setState(() {
      _selection = _selection.toggle(asset.assetId);
      _isSelecting = _selection.selectedCount(totalItems) > 0;
    });
  }

  void _selectAll(LibraryState state) {
    if (_totalItems(state) == 0) {
      return;
    }
    setState(() {
      _isSelecting = true;
      _selection = _selection.selectAll();
    });
  }

  void _clearSelection() {
    setState(() {
      _selection = _selection.clear();
      _isSelecting = false;
    });
  }

  void _onSearchChanged(String value) {
    if (_destination != _LibraryDestination.gallery) {
      setState(() => _destination = _LibraryDestination.gallery);
    }
    _searchDebounce?.cancel();
    _searchDebounce = Timer(const Duration(milliseconds: 250), () {
      if (!mounted) {
        return;
      }
      final state = ref.read(libraryControllerProvider);
      _applyQuery(state.query.copyWith(searchText: value));
    });
  }

  Future<LibraryQueryUpdateOutcome> _applyQuery(
    LibraryGalleryQuery query, {
    BigInt? synchronizationRevision,
    bool preserveGalleryPosition = true,
  }) async {
    final requestGeneration = ++_queryTransitionGeneration;
    final currentState = ref.read(libraryControllerProvider);
    final isContinuingQueryTransition = _pendingQueryPosition != null;
    final frozenPosition = preserveGalleryPosition
        ? _freezeCurrentGalleryPosition(
            currentState,
            allowPreviousQueryIdentity: isContinuingQueryTransition,
          )
        : null;
    _pendingQueryPosition = frozenPosition;
    final viewerAnchor =
        synchronizationRevision == null || _viewerAssetId == null
        ? null
        : _assetByStableIdentity(
                currentState.assets,
                _viewerAssetId!,
                _viewerLocationId,
              ) ??
              _retainedViewerAsset;
    final requestedAnchorLocationId =
        viewerAnchor?.locationId ?? frozenPosition?.locationId;
    final anchorAssetId =
        viewerAnchor?.assetId ??
        frozenPosition?.assetId ??
        (frozenPosition == null
            ? null
            : _assetByLocation(
                currentState.assets,
                frozenPosition.locationId,
              )?.assetId);
    final controller = ref.read(libraryControllerProvider.notifier);
    final viewerIndex = viewerAnchor == null
        ? -1
        : currentState.assets.indexWhere(
            (asset) =>
                asset.assetId == viewerAnchor.assetId &&
                asset.locationId == viewerAnchor.locationId,
          );
    final fallbackGlobalItemIndex = viewerIndex >= 0
        ? currentState.windowStartItemOffset + viewerIndex
        : frozenPosition?.globalItemIndex;
    final updateOutcome = synchronizationRevision == null
        ? (await controller.updateQuery(
                query,
                anchorLocationId: requestedAnchorLocationId,
                anchorAssetId: anchorAssetId,
                fallbackGlobalItemIndex: fallbackGlobalItemIndex,
              )
              ? LibraryQueryUpdateOutcome.applied
              : LibraryQueryUpdateOutcome.failed)
        : await controller.refreshFromSynchronization(
            catalogRevision: synchronizationRevision,
            anchorLocationId: requestedAnchorLocationId,
            anchorAssetId: anchorAssetId,
            fallbackGlobalItemIndex: fallbackGlobalItemIndex,
          );
    if (!mounted || requestGeneration != _queryTransitionGeneration) {
      return LibraryQueryUpdateOutcome.superseded;
    }
    if (updateOutcome != LibraryQueryUpdateOutcome.applied) {
      _pendingQueryPosition = null;
      _scheduleRecoveredDimensionPublication();
      return updateOutcome;
    }
    if (synchronizationRevision != null) {
      await _reconcileViewerAfterSynchronization();
      if (!mounted || requestGeneration != _queryTransitionGeneration) {
        return LibraryQueryUpdateOutcome.superseded;
      }
    }
    final state = ref.read(libraryControllerProvider);
    final revision = state.catalogRevision;
    final resolution = state.queryAnchorResolution;
    LibraryGalleryVisiblePosition? nextPosition;
    if (revision != null && state.assets.isNotEmpty) {
      final frozenLoadedIndex = frozenPosition == null
          ? -1
          : state.assets.indexWhere(
              (asset) => asset.locationId == frozenPosition.locationId,
            );
      final didReturnToFrozenQuery =
          frozenPosition != null &&
          frozenPosition.queryId == state.queryId &&
          frozenPosition.revision == revision &&
          frozenLoadedIndex >= 0 &&
          state.windowStartItemOffset + frozenLoadedIndex ==
              frozenPosition.globalItemIndex;
      if (didReturnToFrozenQuery) {
        nextPosition = frozenPosition;
      } else if (frozenPosition != null &&
          resolution != null &&
          resolution.requestedLocationId == frozenPosition.locationId &&
          resolution.didResolve) {
        final resolvedAsset = _assetByLocation(
          state.assets,
          resolution.locationId!,
        );
        nextPosition = LibraryGalleryVisiblePosition(
          queryId: state.queryId,
          revision: revision,
          monthKey: null,
          locationId: resolution.locationId!,
          assetId: resolvedAsset?.assetId,
          globalItemIndex: resolution.ordinal!,
          itemFraction: frozenPosition.itemFraction,
          viewportFraction: frozenPosition.viewportFraction,
        );
      } else {
        nextPosition = LibraryGalleryVisiblePosition(
          queryId: state.queryId,
          revision: revision,
          monthKey: state.timeline?.buckets.firstOrNull?.monthKey,
          locationId: state.assets.first.locationId,
          assetId: state.assets.first.assetId,
          globalItemIndex: state.windowStartItemOffset,
          itemFraction: 0,
          viewportFraction: 0,
        );
      }
    }
    _galleryLayoutSnapshot.value = null;
    setState(() {
      _pendingQueryPosition = null;
      _visibleGalleryPosition = nextPosition;
      if (!preserveGalleryPosition || nextPosition == null) {
        _galleryLayoutTransition = null;
      } else {
        _galleryLayoutTransitionGeneration += 1;
        _galleryLayoutTransition = LibraryGalleryLayoutTransition(
          generation: _galleryLayoutTransitionGeneration,
          position: nextPosition,
        );
      }
    });
    if (!preserveGalleryPosition) {
      _scheduleGalleryStartReset(requestGeneration);
    }
    return LibraryQueryUpdateOutcome.applied;
  }

  void _scheduleGalleryStartReset(int requestGeneration) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || requestGeneration != _queryTransitionGeneration) {
        return;
      }
      for (final position in _galleryScrollPositions) {
        if ((position.pixels - position.minScrollExtent).abs() >= 0.5) {
          position.jumpTo(position.minScrollExtent);
        }
      }
    });
  }

  Future<void> _reconcileViewerAfterSynchronization() async {
    final assetId = _viewerAssetId;
    final preferredLocationId = _viewerLocationId;
    if (assetId == null || preferredLocationId == null) {
      return;
    }
    final catalog = ref.read(libraryCatalogProvider);
    final stableAssetCatalog = catalog is LibraryStableAssetCatalog
        ? catalog as LibraryStableAssetCatalog
        : null;
    if (stableAssetCatalog == null) {
      return;
    }
    LibraryAsset? asset;
    try {
      asset = await stableAssetCatalog.loadAssetById(
        assetId: assetId,
        preferredLocationId: preferredLocationId,
      );
    } on Object {
      return;
    }
    if (!mounted ||
        _viewerAssetId != assetId ||
        _viewerLocationId != preferredLocationId) {
      return;
    }
    if (asset == null) {
      ref
          .read(libraryControllerProvider.notifier)
          .updateViewerPreviewDemand(null);
      setState(() {
        _viewerAssetId = null;
        _viewerLocationId = null;
        _retainedViewerAsset = null;
      });
      return;
    }
    final resolvedAsset = asset;
    ref
        .read(libraryControllerProvider.notifier)
        .updateViewerPreviewDemand(resolvedAsset);
    setState(() {
      _viewerLocationId = resolvedAsset.locationId;
      _retainedViewerAsset = resolvedAsset;
    });
  }

  void _changeLayoutShape(GalleryLayoutShape value) {
    if (value == _layoutShape) {
      return;
    }
    _beginGalleryLayoutTransition(() => _layoutShape = value);
    unawaited(_persistViewPreferences());
  }

  void _changeThumbnailSize(GalleryThumbnailSize value) {
    if (value == _thumbnailSize) {
      return;
    }
    _beginGalleryLayoutTransition(() => _thumbnailSize = value);
    unawaited(_persistViewPreferences());
  }

  void _beginGalleryLayoutTransition(VoidCallback applyGeometryChange) {
    final state = ref.read(libraryControllerProvider);
    final position = _freezeCurrentGalleryPosition(state);
    _visibleGalleryRange = null;
    _layoutDimensionRecoveryRange = null;
    _layoutDimensionRecoveryAnchor = null;
    setState(() {
      if (position != null) {
        _galleryLayoutTransitionGeneration += 1;
        _galleryLayoutTransition = LibraryGalleryLayoutTransition(
          generation: _galleryLayoutTransitionGeneration,
          position: position,
        );
      } else {
        _galleryLayoutTransition = null;
      }
      applyGeometryChange();
    });
  }

  LibraryGalleryVisiblePosition? _freezeCurrentGalleryPosition(
    LibraryState state, {
    bool allowPreviousQueryIdentity = false,
  }) {
    bool matchesCurrentIdentity(LibraryGalleryVisiblePosition position) {
      return position.queryId == state.queryId &&
          position.revision == state.catalogRevision;
    }

    final transitionPosition = _galleryLayoutTransition?.position;
    if (transitionPosition != null &&
        (matchesCurrentIdentity(transitionPosition) ||
            allowPreviousQueryIdentity)) {
      return _positionWithAssetIdentity(transitionPosition, state);
    }
    final pendingPosition = _pendingQueryPosition;
    if (pendingPosition != null &&
        (matchesCurrentIdentity(pendingPosition) ||
            allowPreviousQueryIdentity)) {
      return _positionWithAssetIdentity(pendingPosition, state);
    }
    LibraryGalleryVisiblePosition? resolvedPosition;
    if (_galleryScrollController.hasClients) {
      final scrollPosition = _galleryScrollController.position;
      resolvedPosition = _galleryPositionResolver.resolve(
        queryId: state.queryId,
        revision: state.catalogRevision,
        scrollOffset: scrollPosition.pixels,
        viewportDimension: scrollPosition.viewportDimension,
      );
    }
    if (resolvedPosition != null && matchesCurrentIdentity(resolvedPosition)) {
      return _positionWithAssetIdentity(resolvedPosition, state);
    }
    final visiblePosition = _visibleGalleryPosition;
    return visiblePosition != null && matchesCurrentIdentity(visiblePosition)
        ? _positionWithAssetIdentity(visiblePosition, state)
        : null;
  }

  LibraryGalleryVisiblePosition _positionWithAssetIdentity(
    LibraryGalleryVisiblePosition position,
    LibraryState state,
  ) {
    if (position.assetId != null) {
      return position;
    }
    final asset = _assetByLocation(state.assets, position.locationId);
    if (asset == null) {
      return position;
    }
    return LibraryGalleryVisiblePosition(
      queryId: position.queryId,
      revision: position.revision,
      monthKey: position.monthKey,
      locationId: position.locationId,
      assetId: asset.assetId,
      globalItemIndex: position.globalItemIndex,
      itemFraction: position.itemFraction,
      viewportFraction: position.viewportFraction,
    );
  }

  void _completeGalleryLayoutTransition(int generation) {
    final transition = _galleryLayoutTransition;
    if (!mounted || transition?.generation != generation) {
      return;
    }
    setState(() {
      _visibleGalleryPosition = transition?.position;
      _galleryLayoutTransition = null;
    });
    _scheduleRecoveredDimensionPublication();
  }

  void _changeSortKey(LibraryState state, LibraryGallerySortKey value) {
    final query = state.query.copyWith(sortKey: value);
    unawaited(_applyQuery(query));
    unawaited(_persistViewPreferences(query: query));
  }

  void _changeSortDirection(
    LibraryState state,
    LibraryGallerySortDirection value,
  ) {
    final query = state.query.copyWith(sortDirection: value);
    unawaited(_applyQuery(query));
    unawaited(_persistViewPreferences(query: query));
  }

  Future<void> _persistViewPreferences({LibraryGalleryQuery? query}) async {
    final currentQuery = query ?? ref.read(libraryControllerProvider).query;
    try {
      await ref
          .read(libraryViewPreferenceStoreProvider)
          .saveLibraryViewPreferences(
            LibraryViewPreferences(
              layoutShape: _layoutShape,
              thumbnailSize: _thumbnailSize,
              sortKey: currentQuery.sortKey,
              sortDirection: currentQuery.sortDirection,
            ),
          );
    } on Object catch (error) {
      if (mounted) {
        _publishFailureNotification("无法保存图库显示设置", error);
      }
    }
  }

  void _changeSidebarWidth(double width) {
    final nextWidth = width
        .clamp(ameMinimumSidebarWidth, ameMaximumSidebarWidth)
        .toDouble();
    if ((nextWidth - _sidebarWidth).abs() < 0.1) {
      return;
    }
    setState(() => _sidebarWidth = nextWidth);
  }

  void _beginSidebarResize() {
    if (_isSidebarResizing) {
      return;
    }
    setState(() => _isSidebarResizing = true);
  }

  void _endSidebarResize(AmePreferences preferences, double width) {
    unawaited(_persistSidebarWidth(preferences, width));
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && _isSidebarResizing) {
        setState(() => _isSidebarResizing = false);
      }
    });
  }

  void _cancelSidebarResize() {
    if (_isSidebarResizing) {
      setState(() => _isSidebarResizing = false);
    }
  }

  Future<void> _persistSidebarWidth(
    AmePreferences preferences,
    double width,
  ) async {
    final previousWidth = preferences.sidebarWidth;
    try {
      await ref
          .read(amePreferencesControllerProvider.notifier)
          .update(preferences.copyWith(sidebarWidth: width));
    } on Object catch (error) {
      if (mounted) {
        setState(() => _sidebarWidth = previousWidth);
        _publishFailureNotification("无法保存侧栏宽度", error);
      }
    }
  }

  void _cancelCurrentMode() {
    if (_viewerAssetId != null) {
      _closeViewer();
      return;
    }
    if (_isSelecting) {
      _clearSelection();
      return;
    }
    if (_destination == _LibraryDestination.settings) {
      setState(() => _destination = _LibraryDestination.gallery);
    }
  }

  Future<void> _copyAssetPath(LibraryAsset asset) async {
    try {
      await ref
          .read(libraryPlatformActionsProvider)
          .copyText(asset.displayPath);
      if (mounted) {
        _publishSuccessNotification("已复制文件路径");
      }
    } on Object catch (error) {
      if (mounted) {
        _publishFailureNotification("无法复制路径", error);
      }
    }
  }

  Future<void> _revealAsset(LibraryAsset asset) async {
    try {
      await ref
          .read(libraryPlatformActionsProvider)
          .revealFile(asset.sourcePath);
    } on Object catch (error) {
      if (mounted) {
        _publishFailureNotification("无法在文件资源管理器中打开", error);
      }
    }
  }

  Future<void> _openRoot(LibraryRoot root) async {
    try {
      await ref.read(libraryPlatformActionsProvider).revealDirectory(root.path);
    } on Object catch (error) {
      if (mounted) {
        _publishFailureNotification("无法在文件资源管理器中打开", error);
      }
    }
  }

  Future<void> _openFolder(LibraryRoot root, LibraryFolder folder) async {
    try {
      await ref
          .read(libraryPlatformActionsProvider)
          .revealLibraryFolder(root.path, folder.relativePath);
    } on Object catch (error) {
      if (mounted) {
        _publishFailureNotification("无法在文件资源管理器中打开", error);
      }
    }
  }

  Future<void> _loadFolderBranch(
    LibraryFolderController controller,
    LibraryState state,
    String rootId,
    String parentRelativePath, {
    bool loadMore = false,
  }) async {
    final revision = state.catalogRevision;
    if (revision == null) {
      return;
    }
    await controller.loadBranch(
      catalogRevision: revision,
      rootId: rootId,
      parentRelativePath: parentRelativePath,
      loadMore: loadMore,
    );
  }

  Future<void> _confirmRemoveRoot(LibraryRoot root) async {
    final shouldRemove = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text("从 Ame 中移除文件夹？"),
        content: Text(
          "将停止在 Ame 中显示“${librarySourceName(root.displayPath)}”。"
          "磁盘上的文件夹和图片不会被删除或修改。",
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text(LibraryStrings.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text(LibraryStrings.removeFromAme),
          ),
        ],
      ),
    );
    if (shouldRemove != true || !mounted) {
      return;
    }
    final didRemove = await ref
        .read(libraryControllerProvider.notifier)
        .unregisterRoot(root);
    if (mounted && didRemove) {
      _publishSuccessNotification("已从 Ame 中移除，原图片未作任何修改");
    }
  }

  void _openSettings() {
    setState(() {
      _destination = _LibraryDestination.settings;
      _selection = _selection.clear();
      _isSelecting = false;
    });
  }

  void _selectLibrary(LibraryState state) {
    setState(() => _destination = _LibraryDestination.gallery);
    unawaited(
      _applyQuery(
        state.query.copyWith(rootId: null, folderRelativePath: null),
        preserveGalleryPosition: false,
      ),
    );
  }

  void _selectRoot(LibraryState state, LibraryRoot root) {
    setState(() => _destination = _LibraryDestination.gallery);
    unawaited(
      _applyQuery(
        state.query.copyWith(rootId: root.id, folderRelativePath: null),
        preserveGalleryPosition: false,
      ),
    );
  }

  void _selectFolder(
    LibraryState state,
    LibraryRoot root,
    LibraryFolder folder,
  ) {
    setState(() => _destination = _LibraryDestination.gallery);
    unawaited(
      _applyQuery(
        state.query.copyWith(
          rootId: root.id,
          folderRelativePath: folder.relativePath,
          includeDescendants: true,
        ),
        preserveGalleryPosition: false,
      ),
    );
  }

  Future<void> _loadPreviousPagePreservingPosition(
    LibraryController controller,
  ) async {
    if (_isRestoringPreviousWindow || !_galleryScrollController.hasClients) {
      return;
    }
    final position = _galleryScrollController.position;
    final previousPixels = position.pixels;
    final previousSnapshot = _galleryLayoutSnapshot.value;
    final previousContentExtent = previousSnapshot?.metrics.contentExtent ?? 0;
    final previousLeadingExtent =
        previousSnapshot?.virtualGeometry.leadingExtent ?? 0;
    _isRestoringPreviousWindow = true;
    try {
      final didLoad = await controller.loadPreviousPage();
      if (!didLoad || !mounted) {
        return;
      }
      await Future<void>.delayed(Duration.zero);
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted || !_galleryScrollController.hasClients) {
        return;
      }
      final nextPosition = _galleryScrollController.position;
      final nextSnapshot = _galleryLayoutSnapshot.value;
      final addedExtent =
          (nextSnapshot?.metrics.contentExtent ?? previousContentExtent) -
          previousContentExtent;
      final leadingDelta =
          (nextSnapshot?.virtualGeometry.leadingExtent ??
              previousLeadingExtent) -
          previousLeadingExtent;
      final displacement = addedExtent + leadingDelta;
      if (displacement.abs() > 0.01) {
        nextPosition.jumpTo(
          (previousPixels + displacement)
              .clamp(nextPosition.minScrollExtent, nextPosition.maxScrollExtent)
              .toDouble(),
        );
      }
    } finally {
      _isRestoringPreviousWindow = false;
    }
  }

  Future<bool> _seekTimeline(
    LibraryController controller,
    LibraryTimeBucket bucket,
    int itemOffset,
  ) async {
    final didSeek = await controller.jumpToTime(bucket, itemOffset: itemOffset);
    if (!mounted) {
      return false;
    }
    if (!didSeek) {
      final error = ref
          .read(libraryControllerProvider)
          .timeNavigationErrorMessage;
      if (error != null) {
        _publishFailureNotification("无法跳转到所选日期", error);
      }
    }
    return didSeek;
  }

  void _beginTimelineNavigation() {
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionSettleTimer = null;
    _layoutDimensionDeadlineTimer?.cancel();
    _layoutDimensionDeadlineTimer = null;
    _visibleGalleryRange = null;
    _layoutDimensionRecoveryRange = null;
    _layoutDimensionRecoveryAnchor = null;
  }

  void _handleGalleryLayoutChanged(
    LibraryGalleryLayoutMetrics nextMetrics,
    LibraryVirtualGalleryGeometry nextVirtualGeometry,
  ) {
    if (!mounted) {
      return;
    }
    final currentState = ref.read(libraryControllerProvider);
    if (currentState.windowStartItemOffset !=
            nextVirtualGeometry.windowStartItemOffset ||
        currentState.assets.length != nextVirtualGeometry.loadedItemCount ||
        currentState.queryId != nextVirtualGeometry.queryId) {
      return;
    }
    final previousSnapshot = _galleryLayoutSnapshot.value;
    final previousMetrics = previousSnapshot?.metrics;
    final previousVirtualGeometry = previousSnapshot?.virtualGeometry;
    final hasSameMetrics =
        previousMetrics?.hasSameGeometry(nextMetrics) ?? false;
    if (hasSameMetrics && nextMetrics.isQueryWide) {
      return;
    }
    if (hasSameMetrics &&
        (previousVirtualGeometry?.hasSameGeometry(nextVirtualGeometry) ??
            false)) {
      return;
    }
    _galleryLayoutSnapshot.value = _LibraryGalleryLayoutSnapshot(
      metrics: nextMetrics,
      virtualGeometry: nextVirtualGeometry,
      catalogRevision: currentState.catalogRevision,
    );
    if (_galleryLayoutTransition != null) {
      return;
    }
    if (nextMetrics.isQueryWide) {
      return;
    }
    final anchorItemIndex = _visibleGalleryPosition?.globalItemIndex;
    final previousAnchorOffset = anchorItemIndex == null
        ? null
        : previousMetrics?.offsetForGlobalItemIndex(anchorItemIndex);
    final nextAnchorOffset = anchorItemIndex == null
        ? null
        : nextMetrics.offsetForGlobalItemIndex(anchorItemIndex);
    final previousPixels = _galleryScrollController.hasClients
        ? _galleryScrollController.position.pixels
        : 0.0;
    final previousValue = previousVirtualGeometry?.valueForScrollOffset(
      previousPixels,
    );
    if (_isRestoringPreviousWindow || !_galleryScrollController.hasClients) {
      return;
    }
    final position = _galleryScrollController.position;
    final previousAnchorPixels =
        previousAnchorOffset == null || previousVirtualGeometry == null
        ? null
        : previousAnchorOffset +
              (previousMetrics?.isQueryWide ?? false
                  ? 0
                  : previousVirtualGeometry.leadingExtent);
    final nextAnchorPixels = nextAnchorOffset == null
        ? null
        : nextAnchorOffset +
              (nextMetrics.isQueryWide ? 0 : nextVirtualGeometry.leadingExtent);
    final target = previousAnchorPixels != null && nextAnchorPixels != null
        ? previousPixels + nextAnchorPixels - previousAnchorPixels
        : previousValue == null
        ? previousPixels
        : nextVirtualGeometry.scrollOffsetForValue(previousValue);
    final boundedTarget = target
        .clamp(position.minScrollExtent, position.maxScrollExtent)
        .toDouble();
    if ((position.pixels - boundedTarget).abs() >= 0.5) {
      position.jumpTo(boundedTarget);
    }
  }

  Future<void> _showAssetInformation(LibraryAsset asset) =>
      showLibraryAssetInformation(context, asset);

  LibraryGalleryVisiblePosition? _initialQueryWidePosition(LibraryState state) {
    final revision = state.catalogRevision;
    if (revision == null || state.assets.isEmpty) {
      return null;
    }
    final visiblePosition = _visibleGalleryPosition;
    final loadedEnd = state.windowStartItemOffset + state.assets.length;
    if (visiblePosition != null &&
        visiblePosition.queryId == state.queryId &&
        visiblePosition.revision == revision &&
        visiblePosition.globalItemIndex >= state.windowStartItemOffset &&
        visiblePosition.globalItemIndex < loadedEnd) {
      return _positionWithAssetIdentity(visiblePosition, state);
    }
    return LibraryGalleryVisiblePosition(
      queryId: state.queryId,
      revision: revision,
      monthKey: state.timeline?.buckets.firstOrNull?.monthKey,
      locationId: state.assets.first.locationId,
      assetId: state.assets.first.assetId,
      globalItemIndex: state.windowStartItemOffset,
      itemFraction: 0,
      viewportFraction: 0,
    );
  }

  void _publishSuccessNotification(String message) {
    _notificationController.publish(
      AmeNotificationDraft(
        title: message,
        message: "",
        severity: AmeNotificationSeverity.success,
      ),
    );
  }

  void _publishFailureNotification(String title, Object error) {
    _notificationController.publish(
      AmeNotificationDraft(
        title: title,
        message: AmeNotificationStrings.operationFailed,
        severity: AmeNotificationSeverity.error,
        detail: error.toString(),
        isPersistent: true,
      ),
    );
  }

  static String _queryId(LibraryState state) {
    if (state.queryId.isNotEmpty) {
      return "${state.catalogRevision ?? BigInt.zero}:${state.queryId}";
    }
    return "${state.catalogRevision ?? BigInt.zero}:${state.query.hashCode}";
  }

  static String _stableQueryId(LibraryState state) {
    return state.queryId.isNotEmpty
        ? state.queryId
        : state.query.hashCode.toString();
  }

  static int _totalItems(LibraryState state) {
    return state.timeline?.totalItems ??
        state.roots.fold(0, (sum, root) => sum + root.assetCount);
  }

  static String _galleryTitle(LibraryState state) {
    final folder = state.query.folderRelativePath;
    if (folder != null) {
      return folder.split("/").last;
    }
    final rootId = state.query.rootId;
    if (rootId == null) {
      return LibraryStrings.library;
    }
    for (final root in state.roots) {
      if (root.id == rootId) {
        return librarySourceName(root.displayPath);
      }
    }
    return LibraryStrings.library;
  }

  static LibraryAsset? _assetByLocation(
    List<LibraryAsset> assets,
    String locationId,
  ) {
    for (final asset in assets) {
      if (asset.locationId == locationId) {
        return asset;
      }
    }
    return null;
  }

  static LibraryAsset? _assetByStableIdentity(
    List<LibraryAsset> assets,
    String assetId,
    String? preferredLocationId,
  ) {
    for (final asset in assets) {
      if (asset.assetId == assetId) {
        if (preferredLocationId == null ||
            asset.locationId == preferredLocationId) {
          return asset;
        }
      }
    }
    return null;
  }

  static String? _transientRootPath(LibraryState state) {
    final rootPath = state.displayRootPath;
    if (rootPath == null ||
        state.roots.any((root) => root.displayPath == rootPath)) {
      return null;
    }
    return rootPath;
  }

  static bool _showsTaskSurface(LibraryState state) {
    return switch (state.status) {
      LibraryStatus.empty => false,
      LibraryStatus.completed => state.scanId != null,
      _ => true,
    };
  }
}

enum _LibraryDestination { gallery, settings }

class _LibraryGalleryLayoutSnapshot {
  const _LibraryGalleryLayoutSnapshot({
    required this.metrics,
    required this.virtualGeometry,
    required this.catalogRevision,
  });

  final LibraryGalleryLayoutMetrics metrics;
  final LibraryVirtualGalleryGeometry virtualGeometry;
  final BigInt? catalogRevision;

  bool matches(LibraryState state) {
    if (virtualGeometry.queryId != state.queryId ||
        catalogRevision != state.catalogRevision) {
      return false;
    }
    if (metrics.isQueryWide) {
      return virtualGeometry.totalItemCount ==
          (state.timeline?.totalItems ?? state.assets.length);
    }
    return virtualGeometry.windowStartItemOffset ==
            state.windowStartItemOffset &&
        virtualGeometry.loadedItemCount == state.assets.length;
  }
}
