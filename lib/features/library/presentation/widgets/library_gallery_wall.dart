import "dart:async";

import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/scheduler.dart";

import "../../application/library_controller.dart";
import "../../domain/gallery_layout_manifest.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "../gallery_selection.dart";
import "../gallery_view_options.dart";
import "../library_strings.dart";
import "library_exact_extent_sliver.dart";
import "library_gallery_layout.dart";
import "library_gallery_layout_snapshot.dart";
import "library_photo_tile.dart";
import "library_virtual_gallery_geometry.dart";
import "library_virtual_gallery_placeholder.dart";

class LibraryGalleryVisiblePosition {
  const LibraryGalleryVisiblePosition({
    required this.monthKey,
    required this.locationId,
    required this.globalItemIndex,
  });

  final String? monthKey;
  final String? locationId;
  final int globalItemIndex;
}

class _LibraryGalleryViewportAnchor {
  const _LibraryGalleryViewportAnchor({
    required this.queryId,
    required this.revision,
    required this.locationId,
    required this.globalItemIndex,
    required this.rowFraction,
    required this.viewportFraction,
  });

  final String queryId;
  final BigInt revision;
  final String locationId;
  final int globalItemIndex;
  final double rowFraction;
  final double viewportFraction;
}

enum _LibraryPreviewMovementDirection { backward, idle, forward }

class LibraryGalleryWall extends StatelessWidget {
  const LibraryGalleryWall({
    required this.state,
    required this.controller,
    required this.scrollController,
    required this.layoutShape,
    required this.thumbnailSize,
    required this.selection,
    required this.isSelecting,
    required this.onOpen,
    required this.onToggleSelection,
    required this.onViewInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    required this.onVisiblePositionChanged,
    required this.onLoadPrevious,
    required this.onLayoutChanged,
    this.layoutManifest,
    super.key,
  });

  final LibraryState state;
  final LibraryController controller;
  final ScrollController scrollController;
  final GalleryLayoutShape layoutShape;
  final GalleryThumbnailSize thumbnailSize;
  final GallerySelection selection;
  final bool isSelecting;
  final ValueChanged<LibraryAsset> onOpen;
  final ValueChanged<LibraryAsset> onToggleSelection;
  final ValueChanged<LibraryAsset> onViewInformation;
  final ValueChanged<LibraryAsset> onCopyPath;
  final ValueChanged<LibraryAsset> onRevealFile;
  final ValueChanged<LibraryGalleryVisiblePosition> onVisiblePositionChanged;
  final Future<void> Function() onLoadPrevious;
  final LibraryGalleryLayoutManifest? layoutManifest;
  final void Function(
    LibraryGalleryLayoutMetrics metrics,
    LibraryVirtualGalleryGeometry virtualGeometry,
  )
  onLayoutChanged;

  static const double _maximumViewportExtent = 16384;
  static const double _directionalPreviewViewportCount = 1.5;
  static const double _guardPreviewViewportCount = 0.75;

  static bool _hasUsableViewport(BoxConstraints constraints) {
    return constraints.maxWidth.isFinite &&
        constraints.maxHeight.isFinite &&
        constraints.maxWidth > 0 &&
        constraints.maxHeight > 0 &&
        constraints.maxWidth <= _maximumViewportExtent &&
        constraints.maxHeight <= _maximumViewportExtent;
  }

  static bool _hasUsableAvailableWidth(double value) {
    return value.isFinite && value > 0 && value <= _maximumViewportExtent;
  }

  @override
  Widget build(BuildContext context) {
    final manifest = layoutManifest;
    if (layoutShape == GalleryLayoutShape.equalHeight &&
        manifest != null &&
        manifest.queryId == state.queryId &&
        manifest.revision == state.catalogRevision &&
        manifest.itemCount ==
            (state.timeline?.totalItems ?? state.assets.length)) {
      return _ManifestLibraryGalleryWall(
        state: state,
        controller: controller,
        scrollController: scrollController,
        thumbnailSize: thumbnailSize,
        selection: selection,
        isSelecting: isSelecting,
        manifest: manifest,
        onOpen: onOpen,
        onToggleSelection: onToggleSelection,
        onViewInformation: onViewInformation,
        onCopyPath: onCopyPath,
        onRevealFile: onRevealFile,
        onVisiblePositionChanged: onVisiblePositionChanged,
        onLoadPrevious: onLoadPrevious,
        onLayoutChanged: onLayoutChanged,
      );
    }
    return LayoutBuilder(
      builder: (context, constraints) {
        if (!_hasUsableViewport(constraints)) {
          return const SizedBox.shrink();
        }
        const horizontalPadding = 24.0;
        final availableWidth = constraints.maxWidth - horizontalPadding - 16;
        if (!_hasUsableAvailableWidth(availableWidth)) {
          return const SizedBox.shrink();
        }
        final entries = LibraryGalleryLayoutEntry.build(
          assets: state.assets,
          availableWidth: availableWidth,
          layoutShape: layoutShape,
          thumbnailSize: thumbnailSize,
          sortKey: state.query.sortKey,
        );
        final layoutMetrics = LibraryGalleryLayoutMetrics.fromEntries(
          entries,
          topPadding: 18,
          bottomPadding: 72,
          itemIndexBase: state.windowStartItemOffset,
        );
        final entryStartOffsets = _entryStartOffsets(entries);
        final estimatedGeometry = LibraryVirtualGalleryGeometry.calculate(
          timeline: state.timeline,
          availableWidth: availableWidth,
          viewportExtent: constraints.maxHeight,
          layoutShape: layoutShape,
          thumbnailSize: thumbnailSize,
          sortKey: state.query.sortKey,
          loadedContentExtent: layoutMetrics.contentExtent,
          windowStartItemOffset: state.windowStartItemOffset,
          loadedItemCount: state.assets.length,
          queryId: state.queryId,
        );
        final virtualGeometry = layoutShape == GalleryLayoutShape.equalHeight
            ? LibraryVirtualGalleryGeometry(
                totalContentExtent: layoutMetrics.contentExtent,
                viewportExtent: constraints.maxHeight,
                leadingExtent: 0,
                loadedContentExtent: layoutMetrics.contentExtent,
                trailingExtent: 0,
                windowStartItemOffset: state.windowStartItemOffset,
                loadedItemCount: state.assets.length,
                totalItemCount:
                    state.timeline?.totalItems ?? state.assets.length,
                queryId: state.queryId,
              )
            : estimatedGeometry;
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!context.mounted) {
            return;
          }
          onLayoutChanged(layoutMetrics, virtualGeometry);
          if (scrollController.hasClients) {
            final metrics = scrollController.position;
            _updatePreviewDemand(
              controller: controller,
              state: state,
              layoutMetrics: layoutMetrics,
              scrollOffset: metrics.pixels - virtualGeometry.leadingExtent,
              viewportDimension: metrics.viewportDimension,
              direction: _LibraryPreviewMovementDirection.idle,
            );
          }
        });
        return ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
          child: Stack(
            children: [
              Listener(
                onPointerSignal: (event) {
                  if (event is! PointerScrollEvent ||
                      !scrollController.hasClients) {
                    return;
                  }
                  if (event.scrollDelta.dy < 0 &&
                      _isNearLoadedStart(
                        scrollController.position,
                        virtualGeometry,
                      ) &&
                      state.hasPreviousAssets &&
                      !state.isLoadingPage &&
                      !state.isLoadingPreviousPage &&
                      state.previousPageErrorMessage == null) {
                    _deferPageRequest(context, onLoadPrevious);
                  } else if (event.scrollDelta.dy > 0 &&
                      _isNearLoadedEnd(
                        scrollController.position,
                        virtualGeometry,
                      ) &&
                      state.hasMoreAssets &&
                      !state.isLoadingPage &&
                      !state.isLoadingPreviousPage &&
                      state.pageErrorMessage == null) {
                    _deferPageRequest(context, controller.loadNextPage);
                  }
                },
                child: NotificationListener<ScrollNotification>(
                  onNotification: (notification) {
                    final isPositionUpdate =
                        notification is ScrollUpdateNotification ||
                        notification is ScrollEndNotification;
                    if (isPositionUpdate) {
                      _updatePreviewDemand(
                        controller: controller,
                        state: state,
                        layoutMetrics: layoutMetrics,
                        scrollOffset:
                            notification.metrics.pixels -
                            virtualGeometry.leadingExtent,
                        viewportDimension:
                            notification.metrics.viewportDimension,
                        direction: _previewDirectionFor(notification),
                      );
                      final position = _positionAtOffset(
                        entries,
                        entryStartOffsets,
                        layoutMetrics,
                        notification.metrics.pixels -
                            virtualGeometry.leadingExtent,
                        notification.metrics.viewportDimension,
                        state.windowStartItemOffset,
                      );
                      if (position != null) {
                        onVisiblePositionChanged(position);
                      }
                      final isDirectDragUpdate =
                          notification is ScrollUpdateNotification &&
                          notification.dragDetails != null;
                      if (isDirectDragUpdate &&
                          _isNearLoadedStart(
                            notification.metrics,
                            virtualGeometry,
                          ) &&
                          state.hasPreviousAssets &&
                          !state.isLoadingPage &&
                          !state.isLoadingPreviousPage &&
                          state.previousPageErrorMessage == null) {
                        _deferPageRequest(context, onLoadPrevious);
                      }
                      if (isDirectDragUpdate &&
                          _isNearLoadedEnd(
                            notification.metrics,
                            virtualGeometry,
                          ) &&
                          state.hasMoreAssets &&
                          !state.isLoadingPage &&
                          !state.isLoadingPreviousPage &&
                          state.pageErrorMessage == null) {
                        _deferPageRequest(context, controller.loadNextPage);
                      }
                    }
                    return false;
                  },
                  child: CustomScrollView(
                    key: const Key("library-photo-wall"),
                    controller: scrollController,
                    slivers: [
                      if (virtualGeometry.leadingExtent > 0)
                        SliverToBoxAdapter(
                          child: LibraryVirtualGalleryPlaceholder(
                            key: const Key("library-leading-placeholder"),
                            extent: virtualGeometry.leadingExtent,
                            horizontalPadding: horizontalPadding + 16,
                            targetTileExtent: thumbnailSize.targetExtent,
                          ),
                        ),
                      SliverPadding(
                        padding: const EdgeInsets.fromLTRB(
                          horizontalPadding,
                          18,
                          16,
                          72,
                        ),
                        sliver: LibraryExactExtentSliver.builder(
                          itemStartOffsets: entryStartOffsets,
                          contentExtent: _entryContentExtent(
                            entryStartOffsets,
                            entries.isEmpty ? 0 : entries.last.extent,
                          ),
                          addSemanticIndexes: false,
                          itemBuilder: (context, index) {
                            final entry = entries[index];
                            if (entry.headerLabel case final label?) {
                              return SizedBox(
                                height: entry.extent,
                                child: Align(
                                  alignment: Alignment.centerLeft,
                                  child: Semantics(
                                    header: true,
                                    child: Text(
                                      label,
                                      key: ValueKey(
                                        "gallery-date-${entry.dateKey ?? 'unknown'}",
                                      ),
                                      style: Theme.of(
                                        context,
                                      ).textTheme.titleSmall,
                                    ),
                                  ),
                                ),
                              );
                            }
                            if (entry.cells.isEmpty) {
                              return SizedBox(height: entry.extent);
                            }
                            return SizedBox(
                              height: entry.extent,
                              child: Align(
                                alignment: Alignment.topLeft,
                                child: SizedBox(
                                  height: entry.rowHeight,
                                  child: Row(
                                    children: [
                                      for (
                                        var cellIndex = 0;
                                        cellIndex < entry.cells.length;
                                        cellIndex++
                                      ) ...[
                                        if (cellIndex > 0)
                                          const SizedBox(
                                            width: LibraryGalleryLayoutEntry
                                                .spacing,
                                          ),
                                        LibraryPhotoTile(
                                          key: ValueKey(
                                            entry
                                                .cells[cellIndex]
                                                .asset
                                                .locationId,
                                          ),
                                          asset: entry.cells[cellIndex].asset,
                                          width: entry.cells[cellIndex].width,
                                          height: entry.rowHeight,
                                          isSelecting: isSelecting,
                                          isSelected: selection.contains(
                                            entry
                                                .cells[cellIndex]
                                                .asset
                                                .locationId,
                                          ),
                                          onOpen: onOpen,
                                          onToggleSelection: onToggleSelection,
                                          onViewInformation: onViewInformation,
                                          onCopyPath: onCopyPath,
                                          onRevealFile: onRevealFile,
                                        ),
                                      ],
                                    ],
                                  ),
                                ),
                              ),
                            );
                          },
                        ),
                      ),
                      if (virtualGeometry.trailingExtent > 0)
                        SliverToBoxAdapter(
                          child: LibraryVirtualGalleryPlaceholder(
                            key: const Key("library-trailing-placeholder"),
                            extent: virtualGeometry.trailingExtent,
                            horizontalPadding: horizontalPadding + 16,
                            targetTileExtent: thumbnailSize.targetExtent,
                          ),
                        ),
                      if (!virtualGeometry.isVirtualized &&
                          state.pageErrorMessage != null)
                        SliverToBoxAdapter(
                          child: SizedBox(
                            height: 72,
                            child: _buildNextPageStatus(),
                          ),
                        ),
                    ],
                  ),
                ),
              ),
              if (state.previousPageErrorMessage != null)
                Positioned(
                  top: 8,
                  left: 0,
                  right: 0,
                  child: Center(
                    child: Material(
                      elevation: 2,
                      color: Theme.of(context).colorScheme.surfaceContainer,
                      borderRadius: BorderRadius.circular(20),
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 14,
                          vertical: 8,
                        ),
                        child: TextButton.icon(
                          key: const Key("library-load-previous-button"),
                          onPressed: onLoadPrevious,
                          icon: const Icon(Icons.refresh),
                          label: const Text(LibraryStrings.retryLoading),
                        ),
                      ),
                    ),
                  ),
                ),
              if (virtualGeometry.isVirtualized &&
                  state.pageErrorMessage != null)
                Positioned(
                  left: 0,
                  right: 0,
                  bottom: 12,
                  child: Center(
                    child: Material(
                      elevation: 2,
                      color: Theme.of(context).colorScheme.surfaceContainer,
                      borderRadius: BorderRadius.circular(20),
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 14,
                          vertical: 8,
                        ),
                        child: _buildNextPageStatus(),
                      ),
                    ),
                  ),
                ),
            ],
          ),
        );
      },
    );
  }

  Widget _buildNextPageStatus() {
    return Center(
      child: OutlinedButton.icon(
        key: const Key("library-load-more-button"),
        onPressed: controller.loadNextPage,
        icon: const Icon(Icons.refresh),
        label: const Text(LibraryStrings.retryLoading),
      ),
    );
  }

  static bool _isNearLoadedStart(
    ScrollMetrics metrics,
    LibraryVirtualGalleryGeometry geometry,
  ) {
    return metrics.pixels >=
            geometry.leadingExtent - metrics.viewportDimension &&
        metrics.pixels <= geometry.leadingExtent + 900;
  }

  static bool _isNearLoadedEnd(
    ScrollMetrics metrics,
    LibraryVirtualGalleryGeometry geometry,
  ) {
    final visibleBottom = metrics.pixels + metrics.viewportDimension;
    return visibleBottom >= geometry.loadedEndExtent - 900 &&
        metrics.pixels <= geometry.loadedEndExtent;
  }

  static void _deferPageRequest(
    BuildContext context,
    Future<void> Function() request,
  ) {
    unawaited(
      Future<void>(() async {
        if (context.mounted) {
          await request();
        }
      }),
    );
  }

  static LibraryGalleryVisiblePosition? _positionAtOffset(
    List<LibraryGalleryLayoutEntry> entries,
    List<double> entryStartOffsets,
    LibraryGalleryLayoutMetrics metrics,
    double scrollOffset,
    double viewportDimension,
    int windowStartItemOffset,
  ) {
    if (entries.isEmpty) {
      return null;
    }
    final target = scrollOffset + (viewportDimension * 0.5) - 18;
    if (target < 0) {
      return null;
    }
    var lower = 0;
    var upper = entryStartOffsets.length;
    while (lower < upper) {
      final middle = lower + ((upper - lower) >> 1);
      if (entryStartOffsets[middle] <= target) {
        lower = middle + 1;
      } else {
        upper = middle;
      }
    }
    final activeIndex = lower - 1;
    if (activeIndex < 0 ||
        target >=
            entryStartOffsets[activeIndex] + entries[activeIndex].extent) {
      return null;
    }
    final activeEntry = entries[activeIndex];
    return LibraryGalleryVisiblePosition(
      monthKey: activeEntry.monthKey,
      locationId: activeEntry.firstLocationId,
      globalItemIndex:
          windowStartItemOffset + metrics.itemIndexForScrollOffset(target + 18),
    );
  }

  static List<double> _entryStartOffsets(
    List<LibraryGalleryLayoutEntry> entries,
  ) {
    final offsets = <double>[];
    var runningOffset = 0.0;
    for (final entry in entries) {
      offsets.add(runningOffset);
      runningOffset += entry.extent;
    }
    return offsets;
  }

  static double _entryContentExtent(
    List<double> entryStartOffsets,
    double lastEntryExtent,
  ) {
    if (entryStartOffsets.isEmpty) {
      return 0;
    }
    return entryStartOffsets.last + lastEntryExtent;
  }

  static _LibraryPreviewMovementDirection _previewDirectionFor(
    ScrollNotification notification,
  ) {
    if (notification case ScrollUpdateNotification(:final scrollDelta?)) {
      if (scrollDelta > 0) {
        return _LibraryPreviewMovementDirection.forward;
      }
      if (scrollDelta < 0) {
        return _LibraryPreviewMovementDirection.backward;
      }
    }
    return _LibraryPreviewMovementDirection.idle;
  }

  static void _updatePreviewDemand({
    required LibraryController controller,
    required LibraryState state,
    required LibraryGalleryLayoutMetrics layoutMetrics,
    required double scrollOffset,
    required double viewportDimension,
    required _LibraryPreviewMovementDirection direction,
  }) {
    if (state.assets.isEmpty ||
        layoutMetrics.itemCount == 0 ||
        !scrollOffset.isFinite ||
        !viewportDimension.isFinite ||
        viewportDimension <= 0) {
      controller.updateGalleryPreviewDemand();
      return;
    }
    final visibleEndOffset = scrollOffset + viewportDimension;
    if (visibleEndOffset <= 0 || scrollOffset >= layoutMetrics.contentExtent) {
      controller.updateGalleryPreviewDemand();
      return;
    }

    int globalItemAt(double offset) {
      final bounded = offset.clamp(0, layoutMetrics.contentExtent).toDouble();
      return layoutMetrics.itemIndexBase +
          layoutMetrics.itemIndexForScrollOffset(bounded);
    }

    int rowStartAt(double offset) {
      final item = globalItemAt(offset);
      return layoutMetrics.rowStartGlobalItemIndex(item) ?? item;
    }

    int rowEndAt(double offset) {
      final item = globalItemAt(offset);
      return layoutMetrics.rowEndGlobalItemIndexExclusive(item) ?? item + 1;
    }

    List<LibraryAsset> assetsInRange(int start, int end) {
      final loadedStart = state.windowStartItemOffset;
      final loadedEnd = loadedStart + state.assets.length;
      final boundedStart = start.clamp(loadedStart, loadedEnd).toInt();
      final boundedEnd = end.clamp(boundedStart, loadedEnd).toInt();
      if (boundedEnd <= boundedStart) {
        return const [];
      }
      return state.assets.sublist(
        boundedStart - loadedStart,
        boundedEnd - loadedStart,
      );
    }

    final visibleStart = rowStartAt(scrollOffset);
    final visibleEnd = rowEndAt(visibleEndOffset);
    final directionalExtent =
        viewportDimension * _directionalPreviewViewportCount;
    final guardExtent = viewportDimension * _guardPreviewViewportCount;
    final backwardDirectionalStart = rowStartAt(
      scrollOffset - directionalExtent,
    );
    final forwardDirectionalEnd = rowEndAt(
      visibleEndOffset + directionalExtent,
    );
    final backwardGuardStart = rowStartAt(scrollOffset - guardExtent);
    final forwardGuardEnd = rowEndAt(visibleEndOffset + guardExtent);

    final nearDirection = switch (direction) {
      _LibraryPreviewMovementDirection.backward => assetsInRange(
        backwardDirectionalStart,
        visibleStart,
      ),
      _LibraryPreviewMovementDirection.idle ||
      _LibraryPreviewMovementDirection.forward => assetsInRange(
        visibleEnd,
        forwardDirectionalEnd,
      ),
    };
    final guard = switch (direction) {
      _LibraryPreviewMovementDirection.backward => assetsInRange(
        visibleEnd,
        forwardGuardEnd,
      ),
      _LibraryPreviewMovementDirection.forward => assetsInRange(
        backwardGuardStart,
        visibleStart,
      ),
      _LibraryPreviewMovementDirection.idle => [
        ...assetsInRange(backwardGuardStart, visibleStart),
        ...assetsInRange(visibleEnd, forwardGuardEnd),
      ],
    };
    controller.updateGalleryPreviewDemand(
      visible: assetsInRange(visibleStart, visibleEnd),
      nearDirection: nearDirection,
      guard: guard,
    );
  }
}

class _ManifestLibraryGalleryWall extends StatefulWidget {
  const _ManifestLibraryGalleryWall({
    required this.state,
    required this.controller,
    required this.scrollController,
    required this.thumbnailSize,
    required this.selection,
    required this.isSelecting,
    required this.manifest,
    required this.onOpen,
    required this.onToggleSelection,
    required this.onViewInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    required this.onVisiblePositionChanged,
    required this.onLoadPrevious,
    required this.onLayoutChanged,
  });

  final LibraryState state;
  final LibraryController controller;
  final ScrollController scrollController;
  final GalleryThumbnailSize thumbnailSize;
  final GallerySelection selection;
  final bool isSelecting;
  final LibraryGalleryLayoutManifest manifest;
  final ValueChanged<LibraryAsset> onOpen;
  final ValueChanged<LibraryAsset> onToggleSelection;
  final ValueChanged<LibraryAsset> onViewInformation;
  final ValueChanged<LibraryAsset> onCopyPath;
  final ValueChanged<LibraryAsset> onRevealFile;
  final ValueChanged<LibraryGalleryVisiblePosition> onVisiblePositionChanged;
  final Future<void> Function() onLoadPrevious;
  final void Function(
    LibraryGalleryLayoutMetrics metrics,
    LibraryVirtualGalleryGeometry virtualGeometry,
  )
  onLayoutChanged;

  @override
  State<_ManifestLibraryGalleryWall> createState() =>
      _ManifestLibraryGalleryWallState();
}

class _ManifestLibraryGalleryWallState
    extends State<_ManifestLibraryGalleryWall> {
  static const _rowRoundingSlack = 0.5;
  static const _topPadding = 18.0;

  LibraryGalleryLayoutSnapshot? _snapshot;
  LibraryExactExtentLayoutCorrection? _layoutCorrection;
  var _layoutCorrectionGeneration = 0;
  var _isResizeFrameScheduled = false;
  double? _pendingAvailableWidth;
  double? _pendingViewportExtent;
  double? _publishedViewportExtent;
  _LibraryGalleryViewportAnchor? _pendingViewportAnchor;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        if (!LibraryGalleryWall._hasUsableViewport(constraints)) {
          _pendingAvailableWidth = null;
          _pendingViewportExtent = null;
          _pendingViewportAnchor = null;
          _layoutCorrection = null;
          return const SizedBox.shrink();
        }
        const horizontalPadding = 24.0;
        final availableWidth = constraints.maxWidth - horizontalPadding - 16;
        if (!LibraryGalleryWall._hasUsableAvailableWidth(availableWidth)) {
          _pendingAvailableWidth = null;
          _pendingViewportExtent = null;
          _pendingViewportAnchor = null;
          _layoutCorrection = null;
          return const SizedBox.shrink();
        }
        final snapshot = _snapshotFor(availableWidth, constraints.maxHeight);
        final assetsByLocation = {
          for (final asset in widget.state.assets) asset.locationId: asset,
        };
        final loadedGeometry = snapshot.loadedWindowGeometry(
          startItemIndex: widget.state.windowStartItemOffset,
          itemCount: widget.state.assets.length,
        );
        final virtualGeometry = LibraryVirtualGalleryGeometry(
          totalContentExtent: snapshot.metrics.contentExtent,
          viewportExtent: constraints.maxHeight,
          leadingExtent: loadedGeometry.leading,
          loadedContentExtent: loadedGeometry.content,
          trailingExtent: loadedGeometry.trailing,
          windowStartItemOffset: widget.state.windowStartItemOffset,
          loadedItemCount: widget.state.assets.length,
          totalItemCount: widget.manifest.itemCount,
          queryId: widget.state.queryId,
        );
        final isPublishedLayout =
            (snapshot.availableWidth - availableWidth).abs() < 0.01;
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!mounted) {
            return;
          }
          if (isPublishedLayout) {
            widget.onLayoutChanged(snapshot.metrics, virtualGeometry);
            _requestVisibleDetailRows(
              snapshot,
              direction: _LibraryPreviewMovementDirection.idle,
            );
          }
        });
        return ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
          child: Stack(
            children: [
              Listener(
                onPointerSignal: (event) {
                  if (event is! PointerScrollEvent ||
                      !widget.scrollController.hasClients) {
                    return;
                  }
                  if (event.scrollDelta.dy < 0 &&
                      LibraryGalleryWall._isNearLoadedStart(
                        widget.scrollController.position,
                        virtualGeometry,
                      ) &&
                      widget.state.hasPreviousAssets &&
                      !widget.state.isLoadingPage &&
                      !widget.state.isLoadingPreviousPage &&
                      widget.state.previousPageErrorMessage == null) {
                    LibraryGalleryWall._deferPageRequest(
                      context,
                      widget.onLoadPrevious,
                    );
                  } else if (event.scrollDelta.dy > 0 &&
                      LibraryGalleryWall._isNearLoadedEnd(
                        widget.scrollController.position,
                        virtualGeometry,
                      ) &&
                      widget.state.hasMoreAssets &&
                      !widget.state.isLoadingPage &&
                      !widget.state.isLoadingPreviousPage &&
                      widget.state.pageErrorMessage == null) {
                    LibraryGalleryWall._deferPageRequest(
                      context,
                      widget.controller.loadNextPage,
                    );
                  }
                },
                child: NotificationListener<ScrollNotification>(
                  onNotification: (notification) {
                    final isPositionUpdate =
                        notification is ScrollUpdateNotification ||
                        notification is ScrollEndNotification;
                    if (isPositionUpdate) {
                      final position = _positionAtOffset(
                        snapshot,
                        notification.metrics.pixels,
                        notification.metrics.viewportDimension,
                      );
                      if (position != null) {
                        widget.onVisiblePositionChanged(position);
                      }
                      _requestVisibleDetailRows(
                        snapshot,
                        scrollMetrics: notification.metrics,
                        direction: LibraryGalleryWall._previewDirectionFor(
                          notification,
                        ),
                      );
                      final isDirectDragUpdate =
                          notification is ScrollUpdateNotification &&
                          notification.dragDetails != null;
                      if (isDirectDragUpdate &&
                          LibraryGalleryWall._isNearLoadedStart(
                            notification.metrics,
                            virtualGeometry,
                          ) &&
                          widget.state.hasPreviousAssets &&
                          !widget.state.isLoadingPage &&
                          !widget.state.isLoadingPreviousPage &&
                          widget.state.previousPageErrorMessage == null) {
                        LibraryGalleryWall._deferPageRequest(
                          context,
                          widget.onLoadPrevious,
                        );
                      }
                      if (isDirectDragUpdate &&
                          LibraryGalleryWall._isNearLoadedEnd(
                            notification.metrics,
                            virtualGeometry,
                          ) &&
                          widget.state.hasMoreAssets &&
                          !widget.state.isLoadingPage &&
                          !widget.state.isLoadingPreviousPage &&
                          widget.state.pageErrorMessage == null) {
                        LibraryGalleryWall._deferPageRequest(
                          context,
                          widget.controller.loadNextPage,
                        );
                      }
                    }
                    return false;
                  },
                  child: CustomScrollView(
                    key: const Key("library-photo-wall"),
                    controller: widget.scrollController,
                    slivers: [
                      SliverPadding(
                        padding: const EdgeInsets.fromLTRB(
                          horizontalPadding,
                          18,
                          16,
                          72,
                        ),
                        sliver: LibraryExactExtentSliver.builder(
                          itemStartOffsets: snapshot.entryStartOffsets,
                          contentExtent: LibraryGalleryWall._entryContentExtent(
                            snapshot.entryStartOffsets,
                            snapshot.entries.isEmpty
                                ? 0
                                : snapshot.entries.last.extent,
                          ),
                          layoutCorrection: _layoutCorrection,
                          addSemanticIndexes: false,
                          itemBuilder: (context, index) {
                            return _buildEntry(
                              context,
                              snapshot,
                              snapshot.entries[index],
                              assetsByLocation,
                              availableWidth,
                            );
                          },
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              if (widget.state.previousPageErrorMessage != null)
                Positioned(
                  top: 8,
                  left: 0,
                  right: 0,
                  child: Center(
                    child: Material(
                      elevation: 2,
                      color: Theme.of(context).colorScheme.surfaceContainer,
                      borderRadius: BorderRadius.circular(20),
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 14,
                          vertical: 8,
                        ),
                        child: TextButton.icon(
                          key: const Key("library-load-previous-button"),
                          onPressed: widget.onLoadPrevious,
                          icon: const Icon(Icons.refresh),
                          label: const Text(LibraryStrings.retryLoading),
                        ),
                      ),
                    ),
                  ),
                ),
              if (widget.state.pageErrorMessage != null)
                Positioned(
                  left: 0,
                  right: 0,
                  bottom: 12,
                  child: Center(
                    child: Material(
                      elevation: 2,
                      color: Theme.of(context).colorScheme.surfaceContainer,
                      borderRadius: BorderRadius.circular(20),
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 14,
                          vertical: 8,
                        ),
                        child: OutlinedButton.icon(
                          key: const Key("library-load-more-button"),
                          onPressed: widget.controller.loadNextPage,
                          icon: const Icon(Icons.refresh),
                          label: const Text(LibraryStrings.retryLoading),
                        ),
                      ),
                    ),
                  ),
                ),
            ],
          ),
        );
      },
    );
  }

  LibraryGalleryLayoutSnapshot _snapshotFor(
    double availableWidth,
    double viewportExtent,
  ) {
    final current = _snapshot;
    final publishedViewportExtent = _publishedViewportExtent;
    if (current != null &&
        current.matches(
          otherManifest: widget.manifest,
          otherAvailableWidth: availableWidth,
          otherThumbnailSize: widget.thumbnailSize,
          otherSortKey: widget.state.query.sortKey,
        ) &&
        publishedViewportExtent != null &&
        (publishedViewportExtent - viewportExtent).abs() < 0.01) {
      return current;
    }
    if (current != null &&
        LibraryGalleryWall._hasUsableAvailableWidth(current.availableWidth) &&
        current.matchesInputs(
          otherManifest: widget.manifest,
          otherThumbnailSize: widget.thumbnailSize,
          otherSortKey: widget.state.query.sortKey,
        )) {
      _scheduleResizeSnapshot(availableWidth, viewportExtent);
      return current;
    }
    _pendingAvailableWidth = null;
    _pendingViewportExtent = null;
    _pendingViewportAnchor = null;
    _layoutCorrection = null;
    _publishedViewportExtent = viewportExtent;
    return _snapshot = LibraryGalleryLayoutSnapshot.build(
      manifest: widget.manifest,
      availableWidth: availableWidth,
      thumbnailSize: widget.thumbnailSize,
      sortKey: widget.state.query.sortKey,
    );
  }

  void _requestVisibleDetailRows(
    LibraryGalleryLayoutSnapshot snapshot, {
    ScrollMetrics? scrollMetrics,
    required _LibraryPreviewMovementDirection direction,
  }) {
    if (widget.state.assets.isEmpty) {
      return;
    }
    final metrics =
        scrollMetrics ??
        (widget.scrollController.hasClients
            ? widget.scrollController.position
            : null);
    if (metrics == null ||
        !metrics.hasContentDimensions ||
        metrics.viewportDimension <= 0) {
      return;
    }
    final layoutMetrics = snapshot.metrics;
    final visibleStart = metrics.pixels
        .clamp(0, layoutMetrics.contentExtent)
        .toDouble();
    final visibleEnd = (metrics.pixels + metrics.viewportDimension)
        .clamp(0, layoutMetrics.contentExtent)
        .toDouble();
    final firstVisibleItem = layoutMetrics.itemIndexForScrollOffset(
      visibleStart,
    );
    final lastVisibleItem = layoutMetrics.itemIndexForScrollOffset(visibleEnd);
    final firstVisibleRowStart = layoutMetrics.rowStartGlobalItemIndex(
      firstVisibleItem,
    );
    final lastVisibleRowEnd = layoutMetrics.rowEndGlobalItemIndexExclusive(
      lastVisibleItem,
    );
    if (firstVisibleRowStart == null || lastVisibleRowEnd == null) {
      return;
    }
    LibraryGalleryWall._updatePreviewDemand(
      controller: widget.controller,
      state: widget.state,
      layoutMetrics: layoutMetrics,
      scrollOffset: metrics.pixels,
      viewportDimension: metrics.viewportDimension,
      direction: direction,
    );
    if (widget.state.isLoadingPage || widget.state.isLoadingPreviousPage) {
      return;
    }
    widget.controller.ensureVisibleRange(
      startItemOffset: firstVisibleRowStart,
      endItemOffsetExclusive: lastVisibleRowEnd,
    );
  }

  void _scheduleResizeSnapshot(double availableWidth, double viewportExtent) {
    if (!LibraryGalleryWall._hasUsableAvailableWidth(availableWidth) ||
        !viewportExtent.isFinite ||
        viewportExtent <= 0) {
      return;
    }
    _pendingAvailableWidth = availableWidth;
    _pendingViewportExtent = viewportExtent;
    final current = _snapshot;
    _pendingViewportAnchor ??= current == null
        ? null
        : _captureViewportAnchor(current);
    if (_isResizeFrameScheduled) {
      return;
    }
    _isResizeFrameScheduled = true;
    SchedulerBinding.instance.scheduleFrameCallback((_) {
      _isResizeFrameScheduled = false;
      if (!mounted) {
        return;
      }
      final pendingWidth = _pendingAvailableWidth;
      final pendingViewportExtent = _pendingViewportExtent;
      final pendingAnchor = _pendingViewportAnchor;
      _pendingAvailableWidth = null;
      _pendingViewportExtent = null;
      _pendingViewportAnchor = null;
      final current = _snapshot;
      final publishedViewportExtent = _publishedViewportExtent;
      if (pendingWidth == null ||
          pendingViewportExtent == null ||
          current == null ||
          (current.matches(
                otherManifest: widget.manifest,
                otherAvailableWidth: pendingWidth,
                otherThumbnailSize: widget.thumbnailSize,
                otherSortKey: widget.state.query.sortKey,
              ) &&
              publishedViewportExtent != null &&
              (publishedViewportExtent - pendingViewportExtent).abs() < 0.01)) {
        return;
      }
      final replacement =
          current.matches(
            otherManifest: widget.manifest,
            otherAvailableWidth: pendingWidth,
            otherThumbnailSize: widget.thumbnailSize,
            otherSortKey: widget.state.query.sortKey,
          )
          ? current
          : LibraryGalleryLayoutSnapshot.build(
              manifest: widget.manifest,
              availableWidth: pendingWidth,
              thumbnailSize: widget.thumbnailSize,
              sortKey: widget.state.query.sortKey,
            );
      final position = widget.scrollController.hasClients
          ? widget.scrollController.position
          : null;
      final target = pendingAnchor == null
          ? null
          : _targetScrollOffsetForAnchor(
              replacement,
              pendingAnchor,
              pendingViewportExtent,
            );
      final delta = target == null || position == null
          ? null
          : target - position.pixels;
      LibraryExactExtentLayoutCorrection? publishedCorrection;
      setState(() {
        _snapshot = replacement;
        _publishedViewportExtent = pendingViewportExtent;
        if (delta == null || delta.abs() < 0.001) {
          _layoutCorrection = null;
        } else {
          _layoutCorrectionGeneration += 1;
          _layoutCorrection = LibraryExactExtentLayoutCorrection(
            generation: _layoutCorrectionGeneration,
            delta: delta,
          );
          publishedCorrection = _layoutCorrection;
        }
      });
      if (publishedCorrection != null) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted && identical(_layoutCorrection, publishedCorrection)) {
            _layoutCorrection = null;
          }
        });
      }
    });
    SchedulerBinding.instance.scheduleFrame();
  }

  _LibraryGalleryViewportAnchor? _captureViewportAnchor(
    LibraryGalleryLayoutSnapshot snapshot,
  ) {
    if (!widget.scrollController.hasClients || snapshot.entries.isEmpty) {
      return null;
    }
    final position = widget.scrollController.position;
    if (!position.hasViewportDimension || position.viewportDimension <= 0) {
      return null;
    }
    const viewportFraction = 0.5;
    final anchorOffset =
        position.pixels + position.viewportDimension * viewportFraction;
    final itemIndex = snapshot.metrics.itemIndexForScrollOffset(anchorOffset);
    final rowOffset = snapshot.metrics.offsetForGlobalItemIndex(itemIndex);
    if (rowOffset == null) {
      return null;
    }
    final entryIndex = snapshot.entryIndexForScrollOffset(
      (rowOffset - _topPadding).clamp(0, double.infinity).toDouble(),
    );
    if (entryIndex < 0) {
      return null;
    }
    final entry = snapshot.entries[entryIndex];
    final rowFraction = entry.rowHeight <= 0
        ? 0.0
        : ((anchorOffset - rowOffset) / entry.rowHeight)
              .clamp(0.0, 1.0)
              .toDouble();
    return _LibraryGalleryViewportAnchor(
      queryId: snapshot.manifest.queryId,
      revision: snapshot.manifest.revision,
      locationId: snapshot.manifest.locationIdAt(itemIndex),
      globalItemIndex: itemIndex,
      rowFraction: rowFraction,
      viewportFraction: viewportFraction,
    );
  }

  double? _targetScrollOffsetForAnchor(
    LibraryGalleryLayoutSnapshot snapshot,
    _LibraryGalleryViewportAnchor anchor,
    double viewportExtent,
  ) {
    if (snapshot.manifest.queryId != anchor.queryId ||
        snapshot.manifest.revision != anchor.revision ||
        snapshot.manifest.itemCount == 0) {
      return null;
    }
    final itemIndex = anchor.globalItemIndex
        .clamp(0, snapshot.manifest.itemCount - 1)
        .toInt();
    final locationId = snapshot.manifest.locationIdAt(itemIndex);
    final resolvedItemIndex = locationId == anchor.locationId
        ? itemIndex
        : anchor.globalItemIndex
              .clamp(0, snapshot.manifest.itemCount - 1)
              .toInt();
    final rowOffset = snapshot.metrics.offsetForGlobalItemIndex(
      resolvedItemIndex,
    );
    if (rowOffset == null) {
      return null;
    }
    final entryIndex = snapshot.entryIndexForScrollOffset(
      (rowOffset - _topPadding).clamp(0, double.infinity).toDouble(),
    );
    if (entryIndex < 0) {
      return null;
    }
    final rowHeight = snapshot.entries[entryIndex].rowHeight;
    final target =
        rowOffset +
        rowHeight * anchor.rowFraction -
        viewportExtent * anchor.viewportFraction;
    final maximum = (snapshot.metrics.contentExtent - viewportExtent)
        .clamp(0, double.infinity)
        .toDouble();
    return target.clamp(0, maximum).toDouble();
  }

  Widget _buildEntry(
    BuildContext context,
    LibraryGalleryLayoutSnapshot snapshot,
    LibraryGalleryLayoutSnapshotEntry entry,
    Map<String, LibraryAsset> assetsByLocation,
    double availableWidth,
  ) {
    if (entry.headerLabel case final label?) {
      return SizedBox(
        height: entry.extent,
        child: Align(
          alignment: Alignment.centerLeft,
          child: Semantics(
            header: true,
            child: Text(
              label,
              key: ValueKey("gallery-date-${entry.dateKey ?? 'unknown'}"),
              style: Theme.of(context).textTheme.titleSmall,
            ),
          ),
        ),
      );
    }
    if (!entry.isPhotoRow) {
      return SizedBox(height: entry.extent);
    }
    final rowPaintWidth = (snapshot.availableWidth + _rowRoundingSlack)
        .clamp(0, LibraryGalleryWall._maximumViewportExtent)
        .toDouble();
    return SizedBox(
      height: entry.extent,
      child: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: availableWidth,
          height: entry.rowHeight,
          child: ClipRect(
            child: OverflowBox(
              alignment: Alignment.topLeft,
              minWidth: rowPaintWidth,
              maxWidth: rowPaintWidth,
              minHeight: entry.rowHeight,
              maxHeight: entry.rowHeight,
              child: Row(
                children: [
                  for (
                    var cellIndex = 0;
                    cellIndex < entry.itemCount;
                    cellIndex++
                  ) ...[
                    if (cellIndex > 0)
                      const SizedBox(width: LibraryGalleryLayoutEntry.spacing),
                    _buildCell(
                      snapshot,
                      entry.startItemIndex + cellIndex,
                      entry.cellWidths[cellIndex],
                      entry.rowHeight,
                      assetsByLocation,
                    ),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildCell(
    LibraryGalleryLayoutSnapshot snapshot,
    int itemIndex,
    double width,
    double height,
    Map<String, LibraryAsset> assetsByLocation,
  ) {
    final locationId = snapshot.manifest.locationIdAt(itemIndex);
    final asset = assetsByLocation[locationId];
    if (asset == null) {
      return _LibraryGalleryStablePlaceholderTile(
        key: ValueKey(locationId),
        width: width,
        height: height,
      );
    }
    return LibraryPhotoTile(
      key: ValueKey(locationId),
      asset: asset,
      width: width,
      height: height,
      isSelecting: widget.isSelecting,
      isSelected: widget.selection.contains(locationId),
      onOpen: widget.onOpen,
      onToggleSelection: widget.onToggleSelection,
      onViewInformation: widget.onViewInformation,
      onCopyPath: widget.onCopyPath,
      onRevealFile: widget.onRevealFile,
    );
  }

  LibraryGalleryVisiblePosition? _positionAtOffset(
    LibraryGalleryLayoutSnapshot snapshot,
    double scrollOffset,
    double viewportDimension,
  ) {
    if (snapshot.entries.isEmpty) {
      return null;
    }
    final target = scrollOffset + viewportDimension * 0.5 - 18;
    final entryIndex = snapshot.entryIndexForScrollOffset(target);
    if (entryIndex < 0) {
      return null;
    }
    final entry = snapshot.entries[entryIndex];
    final itemIndex = entry.startItemIndex;
    return LibraryGalleryVisiblePosition(
      monthKey: entry.monthKey,
      locationId: snapshot.manifest.locationIdAt(itemIndex),
      globalItemIndex: itemIndex,
    );
  }
}

class _LibraryGalleryStablePlaceholderTile extends StatelessWidget {
  const _LibraryGalleryStablePlaceholderTile({
    required this.width,
    required this.height,
    super.key,
  });

  final double width;
  final double height;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: width,
      height: height,
      child: DecoratedBox(
        decoration: BoxDecoration(
          color: Theme.of(
            context,
          ).colorScheme.surfaceContainerHighest.withValues(alpha: 0.72),
          borderRadius: BorderRadius.circular(10),
        ),
      ),
    );
  }
}
