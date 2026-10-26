import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "../../../app/notifications/ame_notification_strings.dart";
import "../../../app/presentation/ame_notifications.dart";
import "../../../app/presentation/ame_scroll_motion.dart";
import "../adapters/windows_library_platform_actions.dart";
import "../../settings/application/ame_preferences.dart";
import "../../settings/presentation/ame_settings_page.dart";
import "../application/library_browse_admission.dart";
import "../application/library_album_pages.dart";
import "../application/library_catalog.dart";
import "../application/library_controller.dart";
import "../application/library_folder_controller.dart";
import "../application/library_layout_manifest_catalog.dart";
import "../application/library_organization_actions.dart";
import "../application/library_organization_publication.dart";
import "../application/library_organization_conflicts.dart";
import "../application/library_review.dart";
import "../application/library_query_projection.dart";
import "../application/library_synchronization.dart";
import "../application/library_update_controller.dart";
import "../application/library_view_preferences.dart";
import "../application/library_workbench_controller.dart";
import "../domain/gallery_layout_manifest.dart";
import "../domain/library_folder_models.dart";
import "../domain/library_models.dart";
import "../domain/library_organization.dart";
import "../domain/library_state.dart";
import "../domain/library_synchronization_models.dart";
import "gallery_selection.dart";
import "library_browse_workspaces.dart";
import "library_gallery_dimension_publication.dart";
import "library_gallery_layout_continuity.dart";
import "library_gallery_query_transition.dart";
import "library_organization_mode.dart";
import "library_conflict_notifications.dart";
import "library_pending_source.dart";
import "library_review_query_transition.dart";
import "library_strings.dart";
import "library_synchronization_notifications.dart";
import "library_synchronization_refresh.dart";
import "library_search_availability.dart";
import "library_viewer_session.dart";
import "widgets/library_asset_information_sheet.dart";
import "widgets/library_album_navigation.dart";
import "widgets/library_albums_sidebar.dart";
import "widgets/library_collection_navigation.dart";
import "widgets/library_review_toolbar.dart";
import "widgets/library_review_mode_notice.dart";
import "widgets/library_viewer_organization_controls.dart";
import "widgets/library_gallery_activity.dart";
import "widgets/library_gallery_header.dart";
import "widgets/library_selection_actions.dart";
import "widgets/library_gallery_filter.dart";
import "widgets/library_gallery_layout.dart";
import "widgets/library_gallery_prepend_compensation.dart";
import "widgets/library_gallery_viewport.dart";
import "widgets/library_gallery_scroll_surface.dart";
import "widgets/library_sort_rail_projection.dart";
import "widgets/library_gallery_states.dart";
import "widgets/library_gallery_wall.dart";
import "widgets/library_global_bar.dart";
import "widgets/library_image_viewer.dart";
import "widgets/library_viewer_filmstrip.dart";
import "widgets/library_viewer_gallery_viewport.dart";
import "widgets/library_navigation.dart";
import "widgets/library_workbench.dart";
import "widgets/library_exact_duplicates_page.dart";
import "widgets/library_query_failure_banner.dart";
import "widgets/library_bottom_feedback.dart";
import "widgets/library_time_navigation.dart";
import "widgets/library_viewer_transition.dart";
import "widgets/library_update_dialog.dart";
import "widgets/library_virtual_gallery_geometry.dart";

class UnifiedLibraryScreen extends ConsumerStatefulWidget {
  const UnifiedLibraryScreen({super.key});

  @override
  ConsumerState<UnifiedLibraryScreen> createState() =>
      _UnifiedLibraryScreenState();
}

class _UnifiedLibraryScreenState extends ConsumerState<UnifiedLibraryScreen>
    implements LibraryReviewSurface {
  late final AmeScrollController _galleryScrollController;
  final Set<ScrollPosition> _galleryScrollPositions = {};
  late final LibraryController _libraryController;
  late final AmeNotificationController _notificationController;
  late final LibrarySynchronizationNotifications _synchronizationNotifications;
  late final TextEditingController _searchController;
  late GallerySelection _selection;
  late String _selectionStableQueryId;
  late GalleryLayoutShape _layoutShape;
  late GalleryThumbnailSize _thumbnailSize;
  bool _isSidebarResizing = false;
  late final LibraryViewerSession _viewer;
  late final LibraryOrganizationMode _review;
  late final LibraryBrowseWorkspaces _browseWorkspaces;
  late final LibraryOrganizationPublication _organizationPublication;
  late final LibraryOrganizationProjectionRegistration _organizationProjection;
  late final LibraryConflictNotifications _conflictNotifications;
  int _timelineSemanticsGeneration = 0;
  bool _isSelecting = false;
  LibraryGalleryPrependCompensation? _previousWindowCompensation;
  bool get _isRestoringPreviousWindow =>
      _previousWindowCompensation?.isActive ?? false;
  LibraryWorkspace get _destination =>
      ref.read(libraryWorkbenchControllerProvider);
  late final ValueNotifier<_LibraryGalleryLayoutSnapshot?>
  _galleryLayoutSnapshot;
  LibraryGalleryVisiblePosition? _visibleGalleryPosition;
  LibraryGalleryLayoutTransition? _galleryLayoutTransition;
  final _galleryLayoutContinuity = LibraryGalleryLayoutContinuity();
  late final LibraryGalleryQueryTransition _queryTransition;
  late final LibraryReviewQueryTransition _reviewQueryTransition;
  late final LibraryQueryProjectionRegistration _queryProjection;
  final LibraryGalleryPositionResolver _galleryPositionResolver =
      LibraryGalleryPositionResolver();
  int _galleryLayoutTransitionGeneration = 0;
  Timer? _searchDebounce;
  late final LibraryGalleryDimensionPublication _dimensionPublication;
  StreamSubscription<LibraryGalleryLayoutDimensionUpdate>?
  _layoutDimensionSubscription;
  StreamSubscription<LibrarySynchronizationSnapshot>?
  _synchronizationSubscription;
  late LibrarySynchronizationSnapshot _synchronizationSnapshot;
  late final LibrarySynchronizationRefresh _synchronizationRefresh;
  bool _isDisposing = false;

  @override
  void initState() {
    super.initState();
    _galleryScrollController = AmeScrollController(
      onAttach: _handleGalleryScrollPositionAttached,
      onDetach: _handleGalleryScrollPositionDetached,
    );
    _libraryController = ref.read(libraryControllerProvider.notifier);
    _review = LibraryOrganizationMode(ref.read(libraryReviewProvider), this)
      ..addListener(_handleViewerChanged);
    _browseWorkspaces = LibraryBrowseWorkspaces(
      surface: this,
      review: _review,
      publish: ref.read(libraryWorkbenchControllerProvider.notifier).select,
    );
    _organizationPublication = ref.read(libraryOrganizationPublicationProvider);
    _organizationProjection = _organizationPublication.attach(
      _browseWorkspaces,
    );
    _viewer = LibraryViewerSession(
      readLibrary: () => ref.read(libraryControllerProvider),
      loadPrevious: () => _loadPreviousAssetsForViewer(_libraryController),
      loadNext: _libraryController.loadNextPage,
      catalog: ref.read(libraryCatalogProvider),
      onPreviewDemand: _libraryController.updateViewerPreviewDemand,
      onNavigationError: (error) =>
          _publishFailureNotification("无法切换图片", error),
    )..addListener(_handleViewerChanged);
    _notificationController = ref.read(
      ameNotificationControllerProvider.notifier,
    );
    _conflictNotifications = LibraryConflictNotifications(
      _notificationController,
    );
    _synchronizationNotifications = LibrarySynchronizationNotifications(
      notifications: _notificationController,
      readRoots: () => ref.read(libraryControllerProvider).roots,
    );
    _synchronizationRefresh = LibrarySynchronizationRefresh(
      readState: () => ref.read(libraryControllerProvider),
      refresh: (revision) => _libraryController.refreshFromSynchronization(
        catalogRevision: revision,
      ),
      onFailureChanged: _publishSynchronizationRefreshFailure,
    );
    final state = ref.read(libraryControllerProvider);
    final viewPreferences = ref.read(initialLibraryViewPreferencesProvider);
    _searchController = TextEditingController(text: state.query.searchText);
    _galleryLayoutSnapshot = ValueNotifier(null);
    _selection = GallerySelection.empty(_queryId(state));
    _selectionStableQueryId = _stableQueryId(state);
    _layoutShape = viewPreferences.layoutShape;
    _thumbnailSize = viewPreferences.thumbnailSize;
    _dimensionPublication = LibraryGalleryDimensionPublication(
      readState: () => ref.read(libraryControllerProvider),
      capturePosition: _freezeCurrentGalleryPosition,
      hasPendingTransition: () =>
          _galleryLayoutTransition != null ||
          _queryTransition.pendingPosition != null,
      onUserScrollingChanged: _handleDimensionScrollActivity,
      onPublished: _handleRecoveredDimensionPublication,
    );
    _layoutDimensionSubscription = _libraryController
        .watchLayoutDimensionUpdates()
        .listen(_dimensionPublication.accept);
    final synchronization = ref.read(librarySynchronizationProvider);
    _synchronizationSubscription = synchronization.watch().listen(
      _handleSynchronizationSnapshot,
    );
    _synchronizationSnapshot = synchronization.current;
    _queryTransition = LibraryGalleryQueryTransition(
      readState: () => ref.read(libraryControllerProvider),
      capturePosition: (allowPreviousQueryIdentity) =>
          _freezeCurrentGalleryPosition(
            ref.read(libraryControllerProvider),
            allowPreviousQueryIdentity: allowPreviousQueryIdentity,
          ),
      readViewerAnchor: _readViewerQueryAnchor,
      reconcileViewer: _viewer.reconcile,
      onPublished: _publishQueryPosition,
      onFailed: _dimensionPublication.schedulePublication,
    );
    _queryProjection = _libraryController.queryProjections.attach(
      _queryTransition,
    );
    _reviewQueryTransition = LibraryReviewQueryTransition(
      readState: () => library,
      updateQuery: _libraryController.updateQuery,
      positionTransition: _queryTransition,
    );
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _synchronizationNotifications.accept(_synchronizationSnapshot);
      }
    });
    _synchronizationRefresh.queueRevision(
      _synchronizationSnapshot.catalogRevision,
    );
  }

  @override
  void dispose() {
    _isDisposing = true;
    _organizationPublication.detach(_organizationProjection);
    _galleryLayoutContinuity.clear();
    _previousWindowCompensation?.retire();
    _libraryController.queryProjections.detach(_queryProjection);
    _queryTransition.dispose();
    _viewer.removeListener(_handleViewerChanged);
    _review.removeListener(_handleViewerChanged);
    _browseWorkspaces.dispose();
    _review.dispose();
    _viewer.dispose();
    _searchDebounce?.cancel();
    _dimensionPublication.dispose();
    _synchronizationRefresh.dispose();
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
    _synchronizationNotifications.accept(snapshot);
    setState(() => _synchronizationSnapshot = snapshot);
    _synchronizationRefresh.queueRevision(snapshot.catalogRevision);
  }

  void _publishSynchronizationRefreshFailure(bool hasFailure) {
    if (!mounted) {
      return;
    }
    _synchronizationNotifications.setRefreshFailure(hasFailure);
  }

  void _handleNotificationAction(AmeNotificationEntry notification) {
    if (_conflictNotifications.admit(notification)) {
      unawaited(_showOrganizationConflicts());
      return;
    }
    if (ref
        .read(libraryOrganizationActionsProvider.notifier)
        .handleNotification(notification)) {
      return;
    }
    switch (_synchronizationNotifications.admitAction(notification)) {
      case LibrarySynchronizationRefreshRetry():
        _synchronizationRefresh.retry();
      case LibrarySynchronizationRootUpdate(:final root):
        unawaited(_chooseLibraryRootsToUpdate(root));
      case null:
        break;
    }
  }

  Future<void> _chooseLibraryRootsToUpdate(LibraryRoot initialRoot) async {
    final libraryState = ref.read(libraryControllerProvider);
    if (libraryState.isBusy) {
      return;
    }
    final updateState = ref.read(libraryUpdateControllerProvider);
    final selectedRoots = await showLibraryUpdateDialog(
      context: context,
      roots: libraryState.roots
          .where((root) => root.id != libraryState.retainedScanRootId)
          .toList(),
      activeRootIds: updateState.activeRootIds,
      initialRootId: initialRoot.id,
    );
    if (!mounted || selectedRoots == null || selectedRoots.isEmpty) {
      return;
    }
    ref
        .read(libraryUpdateControllerProvider.notifier)
        .startUpdates(selectedRoots.map((root) => root.id));
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

  void _handleRootRemovalCompleted(int? previous, int next) {
    if (!mounted || next == 0 || previous == next) {
      return;
    }
    final state = ref.read(libraryControllerProvider);
    setState(() {
      _selection = GallerySelection.empty(_queryId(state));
      _selectionStableQueryId = _stableQueryId(state);
      _isSelecting = false;
    });
  }

  void _handleSelectionQueryChanged(
    ({
      String queryId,
      String stableQueryId,
      bool isRemovalCommitted,
      int removalSequence,
    })?
    previous,
    ({
      String queryId,
      String stableQueryId,
      bool isRemovalCommitted,
      int removalSequence,
    })
    next,
  ) {
    if (!mounted || previous == next) {
      return;
    }
    if (next.isRemovalCommitted && previous?.isRemovalCommitted != true) {
      setState(() {
        _selection = GallerySelection.empty(next.queryId);
        _selectionStableQueryId = next.stableQueryId;
        _isSelecting = false;
      });
      return;
    }
    if (next.isRemovalCommitted ||
        previous?.removalSequence != next.removalSequence) {
      return;
    }
    setState(() {
      if (_selectionStableQueryId == next.stableQueryId) {
        _selection = _selection.rebind(next.queryId);
      } else {
        _selection = GallerySelection.empty(next.queryId);
      }
      _selectionStableQueryId = next.stableQueryId;
      _isSelecting = !_selection.isEmpty;
    });
  }

  @override
  Widget build(BuildContext context) {
    ref.listen(libraryConflictStatusProvider, (_, next) {
      final status = next.asData?.value;
      if (status != null) _conflictNotifications.accept(status);
    });
    ref.listen(
      libraryControllerProvider.select((state) => state.catalogRevision),
      (_, _) {
        unawaited(_review.refreshContinuation());
      },
    );
    ref.listen<int>(
      libraryControllerProvider.select(
        (state) => state.rootRemovalCompletionSequence,
      ),
      _handleRootRemovalCompleted,
    );
    ref.listen<
      ({
        String queryId,
        String stableQueryId,
        bool isRemovalCommitted,
        int removalSequence,
      })
    >(
      libraryControllerProvider.select(
        (state) => (
          queryId: _queryId(state),
          stableQueryId: _stableQueryId(state),
          isRemovalCommitted: state.isRemovalCommitted,
          removalSequence: state.rootRemovalCompletionSequence,
        ),
      ),
      _handleSelectionQueryChanged,
    );
    final state = ref.watch(libraryControllerProvider);
    final workspace = ref.watch(libraryWorkbenchControllerProvider);
    final updateState = ref.watch(libraryUpdateControllerProvider);
    final notifications = ref.watch(ameNotificationControllerProvider);
    _dimensionPublication.synchronizeContext(
      state.catalogRevision,
      state.queryId,
    );
    final amePreferences = ref.watch(amePreferencesControllerProvider);
    final controller = _libraryController;
    final catalogRevision = state.catalogRevision;
    final manifestRequest = catalogRevision != null && state.queryId.isNotEmpty
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
        : _dimensionPublication.manifestWithUpdates(baseLayoutManifest);
    final viewerCatalogAsset = _viewer.retainedAsset;
    final viewerNavigation = _viewer.navigation(_totalItems(state));

    return CallbackShortcuts(
      bindings:
          viewerCatalogAsset == null &&
              workspace.isBrowsing &&
              (workspace != LibraryWorkspace.albums ||
                  state.query.collection is! LibraryAllCollection)
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
              LibraryViewerTransition(
                gallery: LibraryViewerGalleryViewport(
                  isViewerOpen: viewerCatalogAsset != null,
                  child: Column(
                    children: [
                      LibraryGlobalBar(
                        isSearchEnabled:
                            workspace.isBrowsing &&
                            (workspace != LibraryWorkspace.albums ||
                                state.query.collection
                                    is! LibraryAllCollection) &&
                            canEditLibrarySearch(state),
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
                          updateState,
                          layoutManifest,
                        ),
                      ),
                    ],
                  ),
                ),
                viewer: viewerCatalogAsset == null
                    ? null
                    : StreamBuilder<void>(
                        stream: controller.watchPreview(
                          viewerCatalogAsset.locationId,
                        ),
                        builder: (context, snapshot) {
                          final viewerAsset = controller.resolvePreview(
                            viewerCatalogAsset,
                          );
                          return LibraryImageViewer(
                            asset: viewerAsset,
                            organizationActions:
                                LibraryViewerOrganizationControls(
                                  target: LibraryOrganizationTarget.fromAsset(
                                    viewerAsset,
                                  ),
                                  mode: _review,
                                ),
                            operationNotice: LibraryViewerOrganizationNotice(
                              mode: _review,
                            ),
                            sourceNotice: _viewer.sourceNotice,
                            onReloadSource: () {
                              _viewer.reloadSource();
                              if (_viewer.retainedAsset case final asset?) {
                                _review.adoptDisplayedAsset(asset);
                              }
                            },
                            filmstrip: LibraryViewerFilmstrip(
                              selection: viewerAsset,
                              onSelected: _openAsset,
                              loadPrevious: () =>
                                  _loadPreviousAssetsForViewer(controller),
                            ),
                            wheelBehavior: amePreferences.viewerWheelBehavior,
                            openBehavior: amePreferences.viewerOpenBehavior,
                            position: viewerNavigation.position,
                            totalItems: _totalItems(state),
                            onPrevious:
                                viewerNavigation.canPrevious && !_review.busy
                                ? () => unawaited(
                                    _navigateViewer(
                                      LibraryViewerDirection.previous,
                                    ),
                                  )
                                : null,
                            onNext: viewerNavigation.canNext && !_review.busy
                                ? () => unawaited(
                                    _navigateViewer(
                                      LibraryViewerDirection.next,
                                    ),
                                  )
                                : null,
                            onBack: _closeViewer,
                            onInformation: () =>
                                _showAssetInformation(viewerAsset),
                            onCopyPath: () => _copyAssetPath(viewerAsset),
                            onRevealFile: () => _revealAsset(viewerAsset),
                          );
                        },
                      ),
              ),
              if (viewerCatalogAsset == null)
                LibraryBottomFeedback(
                  bottomInset: _review.isActive ? 104 : 24,
                  onRetryPrimary: _retryLibraryTask,
                  onNotificationAction: _handleNotificationAction,
                ),
            ],
          ),
        ),
      ),
    );
  }

  void _handleGalleryUserScrollActivityChanged(bool isScrolling) {
    if (!mounted || _isDisposing) {
      return;
    }
    _dimensionPublication.setUserScrolling(isScrolling);
  }

  void _handleDimensionScrollActivity(bool isScrolling) {
    _queryTransition.setUserScrolling(isScrolling);
    if (isScrolling) {
      _libraryController.queryProjections.invalidatePosition();
    }
  }

  void _handleGalleryScrollPositionAttached(ScrollPosition position) {
    _galleryScrollPositions.add(position);
    position.isScrollingNotifier.addListener(_synchronizeGalleryScrollActivity);
    _synchronizeGalleryScrollActivity();
  }

  void _handleGalleryScrollPositionDetached(ScrollPosition position) {
    _galleryLayoutContinuity.clear();
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

  void _handleRecoveredDimensionPublication(
    LibraryGalleryVisiblePosition? anchor,
  ) {
    if (!mounted) {
      return;
    }
    _galleryLayoutContinuity.clear();
    setState(() {
      if (_layoutShape == GalleryLayoutShape.equalHeight && anchor != null) {
        _galleryLayoutTransitionGeneration += 1;
        _galleryLayoutTransition = LibraryGalleryLayoutTransition(
          generation: _galleryLayoutTransitionGeneration,
          position: anchor,
          scrollPosition: _attachedGalleryScrollPosition,
        );
      }
    });
  }

  Widget _buildLibrary(
    BuildContext context,
    LibraryState state,
    LibraryController controller,
    LibraryUpdateState updateState,
    LibraryGalleryLayoutManifest? layoutManifest,
  ) {
    final folderTree = ref.watch(libraryFolderControllerProvider);
    final selection = _selection;
    final folderController = ref.read(libraryFolderControllerProvider.notifier);
    final collection = state.query.collection;
    final favorites = LibraryCollectionNavigation(
      collection: collection,
      isCompact: false,
      onFavorites: !LibraryBrowseAdmission(state).canBrowse
          ? null
          : () => unawaited(
              _applyQuery(
                state.query.copyWith(
                  collection: const LibraryGalleryCollection.favorites(),
                  rootId: null,
                  folderRelativePath: null,
                ),
                preserveGalleryPosition: false,
              ),
            ),
    );
    final albumName = collection is LibraryAlbumCollection
        ? ref.watch(libraryAlbumProvider(collection.albumId)).value?.name
        : null;
    return LibraryWorkbench(
      onWorkspaceSelected: (workspace) {
        _searchDebounce?.cancel();
        unawaited(_browseWorkspaces.select(workspace));
      },
      albumLanding: collection is LibraryAllCollection
          ? const LibraryAlbumsLanding()
          : null,
      albumsNavigationBuilder: (width) => LibraryAlbumsSidebar(
        width: width,
        child: Column(
          children: [
            favorites,
            LibraryAlbumNavigation(
              collection: collection,
              enabled: LibraryBrowseAdmission(state).canBrowse,
              onSelect: (album) => unawaited(
                _applyQuery(
                  state.query.copyWith(
                    collection: LibraryGalleryCollection.album(album.id),
                    rootId: null,
                    folderRelativePath: null,
                  ),
                  preserveGalleryPosition: false,
                ),
              ),
            ),
          ],
        ),
      ),
      navigationBuilder: (width) => LibraryNavigation(
        isCompact: false,
        width: width,
        roots: state.roots,
        rootSynchronizationStatuses: _synchronizationSnapshot.roots,
        hasSynchronizationFailure:
            _synchronizationSnapshot.lastErrorCode != null,
        selectedRootId: state.query.rootId,
        collection: state.query.collection,
        selectedFolderRelativePath: state.query.folderRelativePath,
        transientSource: LibraryPendingSource.fromState(state),
        folderTree: folderTree,
        isBusy: state.isBusy,
        isBrowseDisabled: !LibraryBrowseAdmission(state).canBrowse,
        updatingRootIds: updateState.activeRootIds,
        isAddingSourceDisabled:
            updateState.hasActive ||
            state.hasRetainedScan ||
            state.primaryScanSnapshot.hasUnresolvedPublication,
        addingSourceDisabledReason:
            LibraryPendingSource.addFolderDisabledReason(
              state.primaryScanSnapshot,
            ),
        retainedScanRootId: state.retainedScanRootId,
        onSelectLibrary: () => _selectLibrary(state),
        onSelectRoot: (root) => _selectRoot(state, root),
        onSelectFolder: (root, folder) => _selectFolder(state, root, folder),
        onExpandFolder: (rootId, parentRelativePath) => _loadFolderBranch(
          folderController,
          state,
          rootId,
          parentRelativePath,
        ),
        onLoadMoreFolders: (rootId, parentRelativePath) => _loadFolderBranch(
          folderController,
          state,
          rootId,
          parentRelativePath,
          loadMore: true,
        ),
        onAddSource: _addSource,
        onUpdateRoot: (root) => unawaited(_chooseLibraryRootsToUpdate(root)),
        onRelocateRoot: (root) =>
            unawaited(controller.chooseDirectoryAndRelocate(root)),
        onOpenRoot: _openRoot,
        onOpenFolder: _openFolder,
        onRemoveRoot: _confirmRemoveRoot,
      ),
      galleryBuilder: (_) => LibraryGalleryActivity(
        controller: controller,
        child: LibraryGalleryViewport(
          header: LibraryGalleryHeader(
            isReviewing: _review.isActive,
            reviewToolbar: _review.isActive
                ? LibraryReviewToolbar(
                    mode: _review,
                    counts: state.organizationCounts,
                  )
                : null,
            reviewCounts: _review.isActive ? state.organizationCounts : null,
            onToggleReview:
                _destination != LibraryWorkspace.gallery ||
                    !_review.canEnter ||
                    _review.busy ||
                    !LibraryBrowseAdmission(state).canBrowse
                ? null
                : () => unawaited(_review.toggle()),
            selectionToolbar: state.catalogRevision == null
                ? null
                : LibrarySelectionActions(
                    selection: _selection,
                    query: state.query,
                    catalogRevision: state.catalogRevision!,
                    onCancel: _clearSelection,
                    enabled:
                        _selection.selectedCount(_totalItems(state)) > 0 &&
                        LibraryBrowseAdmission(state).canBrowse &&
                        !state.isRefreshingQuery,
                    onCommitted: () {
                      if (mounted &&
                          _selection.sharesIntentWith(selection) &&
                          _selectionStableQueryId == _stableQueryId(state)) {
                        _clearSelection();
                      }
                    },
                  ),
            filter: LibraryGalleryFilter(
              query: state.query,
              onChanged: (query) {
                unawaited(_applyQuery(query));
                unawaited(_persistViewPreferences(query: query));
              },
            ),
            galleryTitle: _galleryTitle(state, albumName: albumName),
            totalItems: state.timeline?.totalSourceItems ?? _totalItems(state),
            mergedItems:
                state.query.duplicateMode == LibraryGalleryDuplicateMode.merged
                ? state.timeline?.totalItems
                : null,
            selectedCount: _selection.selectedCount(_totalItems(state)),
            isSelecting: _isSelecting,
            layoutShape: _layoutShape,
            thumbnailSize: _thumbnailSize,
            sortKey: state.query.sortKey,
            sortDirection: state.query.sortDirection,
            onBeginSelection: () => setState(() => _isSelecting = true),
            onCancelSelection: _clearSelection,
            onSelectAll: () => _selectAll(state),
            onLayoutShapeChanged: _changeLayoutShape,
            onThumbnailSizeChanged: _changeThumbnailSize,
            onSortKeyChanged: (value) => _changeSortKey(state, value),
            onSortDirectionChanged: (value) =>
                _changeSortDirection(state, value),
          ),
          failure: switch (state.queryActivity) {
            LibraryQueryFailed failure => LibraryQueryFailureBanner(
              onRetry: !LibraryBrowseAdmission(state).canBrowse
                  ? null
                  : () => unawaited(_retryGalleryQuery(failure.requestedQuery)),
            ),
            _ => null,
          },
          notice: _destination == LibraryWorkspace.gallery
              ? LibraryReviewModeNotice(mode: _review)
              : null,
          isEmpty: state.assets.isEmpty,
          loadingDelay: state.isRefreshingQuery || _review.busy
              ? const Duration(milliseconds: 250)
              : Duration.zero,
          isLoading:
              _review.busy ||
              state.isRefreshingQuery ||
              state.isLoadingPage ||
              state.isLoadingPreviousPage ||
              state.isLoadingTimeAnchor ||
              state.isLoadingVisibleRange,
          body: _buildGalleryBody(state, controller, layoutManifest),
        ),
      ),
      duplicatesBuilder: (_) => const LibraryExactDuplicatesPage(),
      settingsBuilder: (_) => AmeSettingsPage(
        hasLibraryRoots: state.roots.isNotEmpty,
        libraryRootIds: {
          for (final root in state.roots)
            if (!state.isRemovalCommitted || root.id != state.removingRootId)
              root.id,
        },
      ),
      onSidebarResizeChanged: (resizing) {
        if (mounted && _isSidebarResizing != resizing) {
          setState(() => _isSidebarResizing = resizing);
        }
      },
      onPreferenceError: _publishFailureNotification,
    );
  }

  Widget _buildGalleryBody(
    LibraryState state,
    LibraryController controller,
    LibraryGalleryLayoutManifest? layoutManifest,
  ) {
    if (state.assets.isEmpty) {
      return EmptyLibraryGallery(
        state: state,
        onImport: controller.chooseDirectoryAndScan,
      );
    }
    return ValueListenableBuilder<_LibraryGalleryLayoutSnapshot?>(
      valueListenable: _galleryLayoutSnapshot,
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
              _queryTransition.pendingPosition == null) {
            _visibleGalleryPosition = position;
          }
        },
        onVisibleRangeChanged: _dimensionPublication.updateVisibleRange,
        onLoadPrevious: () => _loadPreviousPagePreservingPosition(controller),
        onLayoutChanged: _handleGalleryLayoutChanged,
        layoutManifest: layoutManifest,
        initialQueryWidePosition: _initialQueryWidePosition(state),
        layoutTransition: _galleryLayoutTransition,
        onLayoutTransitionApplied: _completeGalleryLayoutTransition,
        positionResolver: _galleryPositionResolver,
      ),
      builder: (context, snapshot, gallery) {
        final activeSnapshot = snapshot?.matches(state) ?? false
            ? snapshot
            : null;
        return LibraryGalleryScrollSurface(
          controller: _galleryScrollController,
          usePositionRail: !state.query.isChronological,
          scrollIdentity: state.query,
          gallery: gallery!,
          sortProjection: LibrarySortRailProjection(
            timeline: state.timeline,
            metrics: activeSnapshot?.metrics,
          ),
          timeNavigation: _viewer.assetId == null && state.query.isChronological
              ? LibraryTimeNavigation(
                  key: ValueKey<int>(_timelineSemanticsGeneration),
                  isLoading: state.isLoadingTimeline,
                  scrollController: _galleryScrollController,
                  layoutMetrics: activeSnapshot?.metrics,
                  timeline: state.timeline,
                  resolvedTimeAnchor: state.activeTimeAnchor,
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
                )
              : null,
        );
      },
    );
  }

  void _handleViewerChanged() {
    if (mounted && !_isDisposing) {
      final asset = _viewer.retainedAsset;
      if (asset != null && _viewer.sourceNotice == null) {
        _review.reconcileDisplayedAsset(asset);
      }
      setState(() {});
    }
  }

  @override
  LibraryState get library => ref.read(libraryControllerProvider);

  @override
  LibraryReviewBrowseState captureBrowse() => LibraryReviewBrowseState(
    query: library.query,
    selection: _selection,
    isSelecting: _isSelecting,
    position: _freezeCurrentGalleryPosition(library),
  );

  @override
  Future<bool> showReviewQuery(
    LibraryGalleryQuery query, {
    LibraryOrganizationTarget? target,
    LibraryGalleryVisiblePosition? position,
  }) async {
    _previousWindowCompensation?.retire();
    final applied = await _reviewQueryTransition.show(
      query,
      target: target,
      position: position,
    );
    if (!mounted || !applied) return false;
    _searchController.text = query.searchText;
    return true;
  }

  @override
  void restoreReviewBrowse(LibraryReviewBrowseState saved) {
    if (!mounted) return;
    setState(() {
      _selection = saved.selection.queryId == _queryId(library)
          ? saved.selection
          : saved.selection.rebind(_queryId(library));
      _selectionStableQueryId = _stableQueryId(library);
      _isSelecting = saved.isSelecting && !_selection.isEmpty;
    });
  }

  @override
  void openReviewImage(LibraryAsset asset) => _viewer.open(asset);

  @override
  void closeReviewImage() => _closeViewer();

  void _openAsset(LibraryAsset asset) {
    if (_review.busy) return;
    _galleryScrollController.stopWheelMotion();
    _review.open(asset);
    _viewer.open(asset);
  }

  Future<void> _navigateViewer(LibraryViewerDirection direction) async {
    if (await _viewer.navigate(direction) && _viewer.retainedAsset != null) {
      _review.adoptDisplayedAsset(_viewer.retainedAsset!);
    }
  }

  void _closeViewer() {
    _review.closeCurrent();
    _timelineSemanticsGeneration += 1;
    _viewer.close();
  }

  Future<void> _loadPreviousAssetsForViewer(
    LibraryController controller,
  ) async {
    if (_galleryScrollController.hasClients && !_isRestoringPreviousWindow) {
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
    bool preserveGalleryPosition = true,
    bool forceRefresh = false,
  }) async {
    if (_review.isActive) {
      return await _review.changeQuery(query)
          ? LibraryQueryUpdateOutcome.applied
          : LibraryQueryUpdateOutcome.failed;
    }
    return _publishGalleryQuery(
      query,
      preserveGalleryPosition: preserveGalleryPosition,
      forceRefresh: forceRefresh,
    );
  }

  Future<void> _retryGalleryQuery(LibraryGalleryQuery query) async {
    if (await _browseWorkspaces.retry(query) || !mounted) return;
    await _applyQuery(query, forceRefresh: true);
  }

  Future<void> _showOrganizationConflicts() async {
    await _browseWorkspaces.select(LibraryWorkspace.gallery);
    if (!mounted || _destination != LibraryWorkspace.gallery) return;
    await _review.showConflicts();
  }

  Future<LibraryQueryUpdateOutcome> _publishGalleryQuery(
    LibraryGalleryQuery query, {
    bool preserveGalleryPosition = true,
    bool forceRefresh = false,
  }) {
    _previousWindowCompensation?.retire();
    final currentQuery = ref.read(libraryControllerProvider).query;
    final retainsFilterContext =
        preserveGalleryPosition &&
        query.rootId == currentQuery.rootId &&
        query.folderRelativePath == currentQuery.folderRelativePath &&
        query.sortKey == currentQuery.sortKey &&
        query.sortDirection == currentQuery.sortDirection;
    Future<LibraryQueryUpdateOutcome> update(LibraryQueryAnchor? anchor) async {
      final controller = ref.read(libraryControllerProvider.notifier);
      final applied = await controller.updateQuery(
        query,
        anchorLocationId: anchor?.requestedLocationId,
        anchorAssetId: anchor?.assetId,
        fallbackGlobalItemIndex: anchor?.fallbackGlobalItemIndex,
        forceRefresh: forceRefresh,
      );
      return applied
          ? LibraryQueryUpdateOutcome.applied
          : LibraryQueryUpdateOutcome.failed;
    }

    if (retainsFilterContext) {
      return _queryTransition.runFilter(update);
    }
    return _queryTransition.run(
      update,
      preservePosition: preserveGalleryPosition,
    );
  }

  LibraryAsset? _readViewerQueryAnchor() {
    final assetId = _viewer.assetId;
    if (assetId == null) {
      return null;
    }
    return _assetByStableIdentity(
          ref.read(libraryControllerProvider).assets,
          assetId,
          _viewer.locationId,
        ) ??
        _viewer.retainedAsset;
  }

  void _publishQueryPosition(LibraryGalleryQueryPublication publication) {
    if (!mounted || _isDisposing) {
      return;
    }
    _previousWindowCompensation?.retire();
    _galleryLayoutContinuity.clear();
    final nextPosition = publication.position;
    _galleryLayoutSnapshot.value = null;
    setState(() {
      _visibleGalleryPosition = nextPosition;
      if (!publication.preservePosition || nextPosition == null) {
        _galleryLayoutTransition = null;
      } else {
        _galleryLayoutTransitionGeneration += 1;
        _galleryLayoutTransition = LibraryGalleryLayoutTransition(
          generation: _galleryLayoutTransitionGeneration,
          position: nextPosition,
          scrollPosition: _attachedGalleryScrollPosition,
        );
      }
    });
    if (!publication.preservePosition) {
      _scheduleGalleryStartReset(publication.generation);
    }
  }

  void _scheduleGalleryStartReset(int requestGeneration) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !_queryTransition.isCurrent(requestGeneration)) {
        return;
      }
      for (final position in _galleryScrollPositions) {
        if ((position.pixels - position.minScrollExtent).abs() >= 0.5) {
          position.jumpTo(position.minScrollExtent);
        }
      }
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
    _previousWindowCompensation?.retire();
    final state = ref.read(libraryControllerProvider);
    final position = _freezeCurrentGalleryPosition(state);
    _dimensionPublication.resetVisibleRange();
    setState(() {
      if (position != null) {
        _galleryLayoutTransitionGeneration += 1;
        _galleryLayoutTransition = _galleryLayoutContinuity.begin(
          generation: _galleryLayoutTransitionGeneration,
          current: position,
          scrollPosition: _attachedGalleryScrollPosition,
        );
      } else {
        _galleryLayoutContinuity.clear();
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

    final transition = _galleryLayoutTransition;
    final transitionPosition = transition?.position;
    if (transition != null &&
        transitionPosition != null &&
        transition.retainsScrollPosition(_attachedGalleryScrollPosition) &&
        (matchesCurrentIdentity(transitionPosition) ||
            allowPreviousQueryIdentity)) {
      return _positionWithAssetIdentity(transitionPosition, state);
    }
    final pendingPosition = _queryTransition.pendingPosition;
    if (pendingPosition != null &&
        (matchesCurrentIdentity(pendingPosition) ||
            allowPreviousQueryIdentity)) {
      return _positionWithAssetIdentity(pendingPosition, state);
    }
    final resolvedPosition = _resolveCurrentGalleryPosition(state);
    if (resolvedPosition != null && matchesCurrentIdentity(resolvedPosition)) {
      return _positionWithAssetIdentity(resolvedPosition, state);
    }
    final visiblePosition = _visibleGalleryPosition;
    return visiblePosition != null && matchesCurrentIdentity(visiblePosition)
        ? _positionWithAssetIdentity(visiblePosition, state)
        : null;
  }

  LibraryGalleryVisiblePosition? _resolveCurrentGalleryPosition(
    LibraryState state,
  ) {
    if (_galleryScrollController.hasClients) {
      final scrollPosition = _galleryScrollController.position;
      return _galleryPositionResolver.resolve(
        queryId: state.queryId,
        revision: state.catalogRevision,
        scrollOffset: scrollPosition.pixels,
        viewportDimension: scrollPosition.viewportDimension,
      );
    }
    return null;
  }

  ScrollPosition? get _attachedGalleryScrollPosition =>
      _galleryScrollController.hasClients
      ? _galleryScrollController.position
      : null;

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
    _galleryLayoutContinuity.complete(generation);
    setState(() {
      _visibleGalleryPosition =
          _resolveCurrentGalleryPosition(ref.read(libraryControllerProvider)) ??
          transition?.position;
      _galleryLayoutTransition = null;
    });
    _dimensionPublication.schedulePublication();
  }

  void _changeSortKey(LibraryState state, LibraryGallerySortKey value) {
    final query = state.query.copyWith(
      sortKey: value,
      sortDirection: value == LibraryGallerySortKey.libraryAdmission
          ? LibraryGallerySortDirection.descending
          : state.query.sortDirection,
    );
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
    final currentQuery =
        _review.ordinaryQuery ??
        query ??
        ref.read(libraryControllerProvider).query;
    try {
      await ref
          .read(libraryViewPreferenceStoreProvider)
          .saveLibraryViewPreferences(
            LibraryViewPreferences(
              layoutShape: _layoutShape,
              thumbnailSize: _thumbnailSize,
              sortKey: currentQuery.sortKey,
              sortDirection: currentQuery.sortDirection,
              duplicateMode: currentQuery.duplicateMode,
            ),
          );
    } on Object catch (error) {
      if (mounted) {
        _publishFailureNotification("无法保存图库显示设置", error);
      }
    }
  }

  void _cancelCurrentMode() {
    if (_viewer.assetId != null) {
      _closeViewer();
      return;
    }
    if (_isSelecting) {
      _clearSelection();
      return;
    }
    if (_destination != LibraryWorkspace.gallery) {
      unawaited(_browseWorkspaces.select(LibraryWorkspace.gallery));
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
        title: const Text("从 Ame 中移除目录？"),
        content: Text(
          "“${librarySourceName(root.displayPath)}”中的图片将不再显示在 Ame 中。"
          "原目录和图片都会保留。",
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
    await _removeRootAfterFeedback(root);
  }

  Future<void> _removeRootAfterFeedback(LibraryRoot root) async {
    final controller = _libraryController;
    final prepared = controller.prepareRootRemoval(root);
    if (prepared == null) {
      return;
    }
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted) {
      controller.abandonRootRemoval(prepared);
      return;
    }
    final didRemove = await controller.executeRootRemoval(prepared);
    if (mounted && didRemove) {
      _publishSuccessNotification(
        "已从 Ame 中移除“${librarySourceName(root.displayPath)}”",
      );
    }
  }

  Future<void> _retryLibraryTask() async {
    final state = ref.read(libraryControllerProvider);
    if (state.taskKind == LibraryTaskKind.remove && !state.isRemovalCommitted) {
      for (final root in state.roots) {
        if (root.id == state.removingRootId) {
          await _removeRootAfterFeedback(root);
          return;
        }
      }
      return;
    }
    await _libraryController.retry();
  }

  void _addSource() {
    ref
        .read(libraryWorkbenchControllerProvider.notifier)
        .select(LibraryWorkspace.gallery);
    unawaited(_libraryController.chooseDirectoryAndScan());
  }

  void _selectLibrary(LibraryState state) {
    ref
        .read(libraryWorkbenchControllerProvider.notifier)
        .select(LibraryWorkspace.gallery);
    unawaited(
      _applyQuery(
        state.query.copyWith(
          collection: const LibraryGalleryCollection.library(),
          rootId: null,
          folderRelativePath: null,
        ),
        preserveGalleryPosition: false,
      ),
    );
  }

  void _selectRoot(LibraryState state, LibraryRoot root) {
    ref
        .read(libraryWorkbenchControllerProvider.notifier)
        .select(LibraryWorkspace.gallery);
    unawaited(
      _applyQuery(
        state.query.copyWith(
          collection: const LibraryGalleryCollection.library(),
          rootId: root.id,
          folderRelativePath: null,
        ),
        preserveGalleryPosition: false,
      ),
    );
  }

  void _selectFolder(
    LibraryState state,
    LibraryRoot root,
    LibraryFolder folder,
  ) {
    ref
        .read(libraryWorkbenchControllerProvider.notifier)
        .select(LibraryWorkspace.gallery);
    unawaited(
      _applyQuery(
        state.query.copyWith(
          collection: const LibraryGalleryCollection.library(),
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
    final previousSnapshot = _galleryLayoutSnapshot.value;
    final compensation = LibraryGalleryPrependCompensation.capture(
      position: position,
      metrics: previousSnapshot?.metrics,
      geometry: previousSnapshot?.virtualGeometry,
      revision: previousSnapshot?.catalogRevision,
    );
    _previousWindowCompensation = compensation;
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
      if (nextSnapshot == null) {
        return;
      }
      final target = compensation.resolve(
        position: nextPosition,
        metrics: nextSnapshot.metrics,
        geometry: nextSnapshot.virtualGeometry,
        revision: nextSnapshot.catalogRevision,
      );
      if (target != null && (target - nextPosition.pixels).abs() > 0.01) {
        nextPosition.jumpTo(target);
      }
    } finally {
      compensation.retire();
      if (identical(_previousWindowCompensation, compensation)) {
        _previousWindowCompensation = null;
      }
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
    _queryTransition.clearFilterPosition();
    _galleryLayoutContinuity.clear();
    _previousWindowCompensation?.retire();
    _dimensionPublication.beginNavigation();
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

  static String _galleryTitle(LibraryState state, {String? albumName}) {
    final collectionName = switch (state.query.collection) {
      LibraryFavoritesCollection() => "收藏夹",
      LibraryAlbumCollection() => albumName ?? "相册",
      LibraryAllCollection() => null,
    };
    final folder = state.query.folderRelativePath;
    if (folder != null) {
      final name = folder.split("/").last;
      return collectionName == null ? name : "$collectionName · $name";
    }
    final rootId = state.query.rootId;
    if (rootId == null) {
      return collectionName ?? LibraryStrings.library;
    }
    for (final root in state.roots) {
      if (root.id == rootId) {
        final name = librarySourceName(root.displayPath);
        return collectionName == null ? name : "$collectionName · $name";
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
}

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
