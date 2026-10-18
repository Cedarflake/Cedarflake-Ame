import "package:flutter/material.dart";
import "package:flutter/scheduler.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../application/library_controller.dart";
import "../../domain/gallery_layout_manifest.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "../gallery_selection.dart";
import "../gallery_view_options.dart";
import "../library_strings.dart";
import "library_exact_extent_sliver.dart";
import "library_gallery_interaction.dart";
import "library_gallery_viewport_feedback.dart";
import "library_gallery_layout.dart";
import "library_gallery_card_metrics.dart";
import "library_gallery_layout_snapshot.dart";
import "library_gallery_layout_motion.dart";
import "library_gallery_motion_entries.dart";
import "library_gallery_position.dart";
import "library_gallery_reflow.dart";
import "library_photo_tile.dart";
import "library_virtual_gallery_geometry.dart";

export "library_gallery_position.dart";
export "library_gallery_viewport_feedback.dart" show LibraryGalleryVisibleRange;

class _LibraryGalleryScrollViewKey extends GlobalObjectKey {
  const _LibraryGalleryScrollViewKey(super.value);

  @override
  String toString() => "[<'library-photo-wall'>]";
}

class LibraryGalleryWall extends StatelessWidget {
  const LibraryGalleryWall({
    required this.state,
    required this.controller,
    required this.scrollController,
    required this.layoutShape,
    required this.thumbnailSize,
    required this.selection,
    required this.isSelecting,
    this.isSidebarResizing = false,
    required this.onOpen,
    required this.onToggleSelection,
    required this.onViewInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    required this.onVisiblePositionChanged,
    this.onVisibleRangeChanged,
    required this.onLoadPrevious,
    required this.onLayoutChanged,
    this.layoutManifest,
    this.initialQueryWidePosition,
    this.layoutTransition,
    this.onLayoutTransitionApplied,
    this.positionResolver,
    super.key,
  });

  final LibraryState state;
  final LibraryController controller;
  final ScrollController scrollController;
  final GalleryLayoutShape layoutShape;
  final GalleryThumbnailSize thumbnailSize;
  final GallerySelection selection;
  final bool isSelecting;
  final bool isSidebarResizing;
  final ValueChanged<LibraryAsset> onOpen;
  final ValueChanged<LibraryAsset> onToggleSelection;
  final ValueChanged<LibraryAsset> onViewInformation;
  final ValueChanged<LibraryAsset> onCopyPath;
  final ValueChanged<LibraryAsset> onRevealFile;
  final ValueChanged<LibraryGalleryVisiblePosition> onVisiblePositionChanged;
  final ValueChanged<LibraryGalleryVisibleRange>? onVisibleRangeChanged;
  final Future<void> Function() onLoadPrevious;
  final LibraryGalleryLayoutManifest? layoutManifest;
  final LibraryGalleryVisiblePosition? initialQueryWidePosition;
  final LibraryGalleryLayoutTransition? layoutTransition;
  final ValueChanged<int>? onLayoutTransitionApplied;
  final LibraryGalleryPositionResolver? positionResolver;
  final void Function(
    LibraryGalleryLayoutMetrics metrics,
    LibraryVirtualGalleryGeometry virtualGeometry,
  )
  onLayoutChanged;

  static const double _maximumViewportExtent = 16384;
  static const double _leadingHorizontalPadding = 24;
  static const double _trailingHorizontalPadding = 16;

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
    final isActive = TickerMode.valuesOf(context).enabled;
    final manifest = layoutManifest;
    if (manifest != null &&
        manifest.queryId == state.queryId &&
        manifest.revision == state.catalogRevision &&
        manifest.itemCount ==
            (state.timeline?.totalItems ?? state.assets.length)) {
      return _ManifestLibraryGalleryWall(
        state: state,
        controller: controller,
        scrollController: scrollController,
        thumbnailSize: thumbnailSize,
        layoutShape: layoutShape,
        selection: selection,
        isSelecting: isSelecting,
        isSidebarResizing: isSidebarResizing,
        manifest: manifest,
        onOpen: onOpen,
        onToggleSelection: onToggleSelection,
        onViewInformation: onViewInformation,
        onCopyPath: onCopyPath,
        onRevealFile: onRevealFile,
        onVisiblePositionChanged: onVisiblePositionChanged,
        onVisibleRangeChanged: onVisibleRangeChanged,
        onLoadPrevious: onLoadPrevious,
        onLayoutChanged: onLayoutChanged,
        initialQueryWidePosition: initialQueryWidePosition,
        layoutTransition: layoutTransition,
        onLayoutTransitionApplied: onLayoutTransitionApplied,
        positionResolver: positionResolver,
      );
    }
    return LayoutBuilder(
      builder: (context, constraints) {
        if (!_hasUsableViewport(constraints)) {
          return const SizedBox.shrink();
        }
        const horizontalPadding = _leadingHorizontalPadding;
        final availableWidth =
            constraints.maxWidth -
            _leadingHorizontalPadding -
            _trailingHorizontalPadding;
        if (!_hasUsableAvailableWidth(availableWidth)) {
          return const SizedBox.shrink();
        }
        final entries = LibraryGalleryLayoutEntry.build(
          minimumCellWidth: LibraryGalleryCardMetrics.minimumWidth,
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
        final devicePixelRatio = MediaQuery.devicePixelRatioOf(context);
        final previewEdgesByLocation = <String, int>{
          for (final entry in entries)
            for (final cell in entry.cells)
              cell.asset.locationId: libraryPreviewDecodeWidth(
                cell.width > entry.rowHeight ? cell.width : entry.rowHeight,
                devicePixelRatio,
              ),
        };
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
        final layoutCorrection = _layoutCorrectionForTransition(
          transition: layoutTransition,
          state: state,
          entries: entries,
          metrics: layoutMetrics,
          virtualGeometry: virtualGeometry,
          viewportExtent: constraints.maxHeight,
          scrollController: scrollController,
          positionResolver: positionResolver,
          fallbackPosition: initialQueryWidePosition,
        );
        positionResolver?.update(
          queryId: state.queryId,
          revision: state.catalogRevision,
          resolve: (scrollOffset, viewportDimension) => _positionAtOffset(
            entries,
            entryStartOffsets,
            scrollOffset - virtualGeometry.leadingExtent,
            viewportDimension,
            state.windowStartItemOffset,
            state.queryId,
            state.catalogRevision,
            availableWidth,
          ),
        );
        void synchronizeVisibleViewport(
          ScrollMetrics metrics,
          LibraryPreviewMovementDirection direction,
        ) {
          if (!context.mounted ||
              !TickerMode.getValuesNotifier(context).value.enabled) {
            return;
          }
          final localScrollOffset =
              metrics.pixels - virtualGeometry.leadingExtent;
          final visiblePosition = _positionAtOffset(
            entries,
            entryStartOffsets,
            localScrollOffset,
            metrics.viewportDimension,
            state.windowStartItemOffset,
            state.queryId,
            state.catalogRevision,
            availableWidth,
          );
          final isViewportInsideLoadedWindow =
              metrics.pixels >= virtualGeometry.leadingExtent &&
              metrics.pixels + metrics.viewportDimension <=
                  virtualGeometry.loadedEndExtent;
          final visibleRange =
              layoutShape == GalleryLayoutShape.square &&
                  virtualGeometry.isVirtualized &&
                  !isViewportInsideLoadedWindow
              ? LibraryGalleryViewportFeedback.estimatedSquareVisibleRangeFor(
                  virtualGeometry: virtualGeometry,
                  scrollOffset: metrics.pixels,
                  viewportDimension: metrics.viewportDimension,
                  availableWidth: availableWidth,
                  thumbnailSize: thumbnailSize,
                  queryId: state.queryId,
                  revision: state.catalogRevision,
                )
              : LibraryGalleryViewportFeedback.visibleRangeFor(
                  layoutMetrics: layoutMetrics,
                  scrollOffset: localScrollOffset,
                  viewportDimension: metrics.viewportDimension,
                  queryId: state.queryId,
                  revision: state.catalogRevision,
                );
          if (visibleRange != null) {
            onVisibleRangeChanged?.call(visibleRange);
            if (layoutShape == GalleryLayoutShape.square) {
              controller.ensureVisibleRange(
                startItemOffset: visibleRange.startGlobalItemIndex,
                endItemOffsetExclusive:
                    visibleRange.endGlobalItemIndexExclusive,
              );
            }
          }
          LibraryGalleryViewportFeedback.updatePreviewDemand(
            controller: controller,
            state: state,
            layoutMetrics: layoutMetrics,
            scrollOffset: localScrollOffset,
            viewportDimension: metrics.viewportDimension,
            direction: direction,
            anchorGlobalItemIndex:
                visiblePosition?.globalItemIndex ?? state.windowStartItemOffset,
            previewEdgeFor: (asset, _) =>
                previewEdgesByLocation[asset.locationId] ?? 512,
          );
          if (visiblePosition != null) {
            onVisiblePositionChanged(visiblePosition);
          }
        }

        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!context.mounted ||
              !isActive ||
              !TickerMode.getValuesNotifier(context).value.enabled) {
            return;
          }
          onLayoutChanged(layoutMetrics, virtualGeometry);
          if (scrollController.hasClients) {
            synchronizeVisibleViewport(
              scrollController.position,
              LibraryPreviewMovementDirection.idle,
            );
          }
        });
        final content = ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
          child: Stack(
            children: [
              LibraryGalleryInteraction(
                state: state,
                scrollController: scrollController,
                geometry: virtualGeometry,
                onLoadPrevious: onLoadPrevious,
                onLoadNext: controller.loadNextPage,
                onMetricsChanged: (metrics) => synchronizeVisibleViewport(
                  metrics,
                  LibraryPreviewMovementDirection.idle,
                ),
                onPositionChanged: synchronizeVisibleViewport,
                child: KeyedSubtree(
                  key: const Key("library-photo-wall"),
                  child: CustomScrollView(
                    clipBehavior: Clip.none,
                    key: _LibraryGalleryScrollViewKey(scrollController),
                    controller: scrollController,
                    slivers: [
                      if (virtualGeometry.leadingExtent > 0)
                        SliverToBoxAdapter(
                          child: SizedBox(
                            key: const Key("library-leading-placeholder"),
                            height: virtualGeometry.leadingExtent,
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
                          layoutCorrection: layoutCorrection,
                          onLayoutCorrectionApplied: (generation) {
                            final transition = layoutTransition;
                            if (transition != null &&
                                generation ==
                                    (
                                      scope: "gallery-transition",
                                      value: transition.generation,
                                    )) {
                              onLayoutTransitionApplied?.call(
                                transition.generation,
                              );
                            }
                          },
                          addSemanticIndexes: false,
                          itemBuilder: (context, index) {
                            final entry = entries[index];
                            final offset =
                                virtualGeometry.leadingExtent +
                                18 +
                                entryStartOffsets[index];
                            if (entry.headerLabel case final label?) {
                              return LibraryGalleryMotionHeader(
                                label: label,
                                dateKey: entry.dateKey,
                                rect: Rect.fromLTWH(
                                  0,
                                  offset,
                                  availableWidth,
                                  entry.extent,
                                ),
                              );
                            }
                            if (entry.cells.isEmpty) {
                              return SizedBox(height: entry.extent);
                            }
                            return LibraryGalleryMotionRow(
                              extent: entry.extent,
                              width: availableWidth,
                              height: entry.rowHeight,
                              offset: offset,
                              cellWidths: [
                                for (final cell in entry.cells) cell.width,
                              ],
                              identityAt: (index) =>
                                  entry.cells[index].asset.locationId,
                              builder: (index, size) {
                                final asset = entry.cells[index].asset;
                                return LibraryPhotoTile(
                                  key: ValueKey(asset.locationId),
                                  asset: asset,
                                  width: size.width,
                                  height: size.height,
                                  isSelecting: isSelecting,
                                  isSelected: selection.contains(asset.assetId),
                                  onOpen: onOpen,
                                  onToggleSelection: onToggleSelection,
                                  onViewInformation: onViewInformation,
                                  onCopyPath: onCopyPath,
                                  onRevealFile: onRevealFile,
                                );
                              },
                            );
                          },
                        ),
                      ),
                      if (virtualGeometry.trailingExtent > 0)
                        SliverToBoxAdapter(
                          child: SizedBox(
                            key: const Key("library-trailing-placeholder"),
                            height: virtualGeometry.trailingExtent,
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
                          icon: const Icon(Symbols.refresh_rounded),
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
        return LibraryGalleryLayoutMotion(
          queryIdentity: (
            state.queryId,
            state.catalogRevision,
            layoutShape,
            thumbnailSize,
          ),
          geometryIdentity: (
            availableWidth,
            constraints.maxHeight,
            layoutShape,
            thumbnailSize,
          ),
          scrollController: scrollController,
          scrollOffsetCorrection: layoutCorrection?.delta ?? 0,
          child: content,
        );
      },
    );
  }

  Widget _buildNextPageStatus() {
    return Center(
      child: OutlinedButton.icon(
        key: const Key("library-load-more-button"),
        onPressed: controller.loadNextPage,
        icon: const Icon(Symbols.refresh_rounded),
        label: const Text(LibraryStrings.retryLoading),
      ),
    );
  }

  static LibraryGalleryVisiblePosition? _positionAtOffset(
    List<LibraryGalleryLayoutEntry> entries,
    List<double> entryStartOffsets,
    double scrollOffset,
    double viewportDimension,
    int windowStartItemOffset,
    String queryId,
    BigInt? revision,
    double availableWidth,
  ) {
    if (entries.isEmpty || revision == null) {
      return null;
    }
    final anchorOffset = scrollOffset + viewportDimension * 0.5;
    final target = anchorOffset - 18;
    final activeIndex = _photoEntryIndexNearestOffset(
      entries,
      entryStartOffsets,
      target,
    );
    if (activeIndex == null) {
      return null;
    }
    final activeEntry = entries[activeIndex];
    final rowOffset = entryStartOffsets[activeIndex] + 18;
    final centerCellIndex = LibraryGalleryViewportAnchor.centerCellIndex([
      for (final cell in activeEntry.cells) cell.width,
    ], availableWidth);
    final itemIndex = activeEntry.startItemIndex + centerCellIndex;
    final fractions = LibraryGalleryAnchorFractions.capture(
      rowOffset: rowOffset,
      rowHeight: activeEntry.rowHeight,
      scrollOffset: scrollOffset,
      viewportExtent: viewportDimension,
      viewportFraction: 0.5,
    );
    return LibraryGalleryVisiblePosition(
      queryId: queryId,
      revision: revision,
      monthKey: activeEntry.monthKey,
      locationId: activeEntry.cells[centerCellIndex].asset.locationId,
      assetId: activeEntry.cells[centerCellIndex].asset.assetId,
      globalItemIndex: windowStartItemOffset + itemIndex,
      itemFraction: fractions.item,
      viewportFraction: fractions.viewport,
    );
  }

  static int? _photoEntryIndexNearestOffset(
    List<LibraryGalleryLayoutEntry> entries,
    List<double> entryStartOffsets,
    double target,
  ) {
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
    final initialIndex = (lower - 1).clamp(0, entries.length - 1).toInt();
    int? previousIndex;
    for (var index = initialIndex; index >= 0; index -= 1) {
      if (entries[index].cells.isNotEmpty && entries[index].rowHeight > 0) {
        previousIndex = index;
        break;
      }
    }
    int? nextIndex;
    for (var index = initialIndex; index < entries.length; index += 1) {
      if (entries[index].cells.isNotEmpty && entries[index].rowHeight > 0) {
        nextIndex = index;
        break;
      }
    }
    if (previousIndex == null && nextIndex == null) {
      return null;
    }

    double distanceToEntry(int index) {
      final top = entryStartOffsets[index];
      final bottom = top + entries[index].rowHeight;
      if (target < top) {
        return top - target;
      }
      if (target > bottom) {
        return target - bottom;
      }
      return 0;
    }

    if (previousIndex == null) {
      return nextIndex;
    }
    if (nextIndex == null) {
      return previousIndex;
    }
    return distanceToEntry(previousIndex) <= distanceToEntry(nextIndex)
        ? previousIndex
        : nextIndex;
  }

  static LibraryExactExtentLayoutCorrection? _layoutCorrectionForTransition({
    required LibraryGalleryLayoutTransition? transition,
    required LibraryState state,
    required List<LibraryGalleryLayoutEntry> entries,
    required LibraryGalleryLayoutMetrics metrics,
    required LibraryVirtualGalleryGeometry virtualGeometry,
    required double viewportExtent,
    required ScrollController scrollController,
    required LibraryGalleryPositionResolver? positionResolver,
    required LibraryGalleryVisiblePosition? fallbackPosition,
  }) {
    if (transition == null ||
        state.catalogRevision == null ||
        transition.position.queryId != state.queryId ||
        transition.position.revision != state.catalogRevision ||
        entries.isEmpty) {
      return null;
    }
    final position = transition.resolvePosition(
      current: scrollController.hasClients ? scrollController.position : null,
      resolver: positionResolver,
      fallback: fallbackPosition,
    );
    if (position == null) {
      return LibraryExactExtentLayoutCorrection(
        generation: (scope: "gallery-transition", value: transition.generation),
        delta: 0,
      );
    }
    final localItemIndex = state.assets.indexWhere(
      (asset) => asset.locationId == position.locationId,
    );
    if (localItemIndex < 0) {
      return null;
    }
    final globalItemIndex = state.windowStartItemOffset + localItemIndex;
    if (globalItemIndex != position.globalItemIndex) {
      return null;
    }
    final rowOffset = metrics.offsetForGlobalItemIndex(globalItemIndex);
    if (rowOffset == null) {
      return null;
    }
    final row = LibraryGalleryLayoutEntry.photoRowForItemIndex(
      entries,
      localItemIndex,
    );
    if (row == null || row.rowHeight <= 0) {
      return null;
    }
    final target =
        virtualGeometry.leadingExtent +
        rowOffset +
        row.rowHeight * position.itemFraction -
        viewportExtent * position.viewportFraction;
    final maximum = (virtualGeometry.totalContentExtent - viewportExtent)
        .clamp(0, double.infinity)
        .toDouble();
    final boundedTarget = transition.constrainScrollOffset(
      current: scrollController.hasClients ? scrollController.position : null,
      imageOffset: target,
      maximum: maximum,
    );
    return LibraryExactExtentLayoutCorrection(
      generation: (scope: "gallery-transition", value: transition.generation),
      delta:
          boundedTarget -
          (scrollController.hasClients
              ? scrollController.position.pixels
              : scrollController.initialScrollOffset),
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
}

class _ManifestLibraryGalleryWall extends StatefulWidget {
  const _ManifestLibraryGalleryWall({
    required this.state,
    required this.controller,
    required this.scrollController,
    required this.thumbnailSize,
    required this.layoutShape,
    required this.selection,
    required this.isSelecting,
    required this.isSidebarResizing,
    required this.manifest,
    required this.onOpen,
    required this.onToggleSelection,
    required this.onViewInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    required this.onVisiblePositionChanged,
    this.onVisibleRangeChanged,
    required this.onLoadPrevious,
    required this.onLayoutChanged,
    required this.initialQueryWidePosition,
    required this.layoutTransition,
    required this.onLayoutTransitionApplied,
    required this.positionResolver,
  });

  final LibraryState state;
  final LibraryController controller;
  final ScrollController scrollController;
  final GalleryThumbnailSize thumbnailSize;
  final GalleryLayoutShape layoutShape;
  final GallerySelection selection;
  final bool isSelecting;
  final bool isSidebarResizing;
  final LibraryGalleryLayoutManifest manifest;
  final ValueChanged<LibraryAsset> onOpen;
  final ValueChanged<LibraryAsset> onToggleSelection;
  final ValueChanged<LibraryAsset> onViewInformation;
  final ValueChanged<LibraryAsset> onCopyPath;
  final ValueChanged<LibraryAsset> onRevealFile;
  final ValueChanged<LibraryGalleryVisiblePosition> onVisiblePositionChanged;
  final ValueChanged<LibraryGalleryVisibleRange>? onVisibleRangeChanged;
  final Future<void> Function() onLoadPrevious;
  final LibraryGalleryVisiblePosition? initialQueryWidePosition;
  final LibraryGalleryLayoutTransition? layoutTransition;
  final ValueChanged<int>? onLayoutTransitionApplied;
  final LibraryGalleryPositionResolver? positionResolver;
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
  LibraryGalleryLayoutSnapshot? _snapshot;
  LibraryExactExtentLayoutCorrection? _layoutCorrection;
  var _layoutCorrectionGeneration = 0;
  var _isResizeFrameScheduled = false;
  double? _publishedViewportExtent;
  LibraryGalleryPendingReflow? _pendingReflow;
  LibraryGalleryViewportAnchor? _sidebarResizeAnchor;
  var _didApplyInitialPosition = false;

  @override
  void didUpdateWidget(covariant _ManifestLibraryGalleryWall oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.manifest.queryId != widget.manifest.queryId ||
        oldWidget.manifest.revision != widget.manifest.revision) {
      _didApplyInitialPosition = false;
      _layoutCorrection = null;
    }
    if (widget.layoutTransition != null &&
        oldWidget.layoutTransition?.generation !=
            widget.layoutTransition?.generation) {
      _didApplyInitialPosition = false;
      _layoutCorrection = null;
    }
    if (!widget.isSidebarResizing ||
        oldWidget.manifest.queryId != widget.manifest.queryId ||
        oldWidget.manifest.revision != widget.manifest.revision ||
        oldWidget.thumbnailSize != widget.thumbnailSize ||
        oldWidget.state.query.sortKey != widget.state.query.sortKey) {
      _sidebarResizeAnchor = null;
    }
  }

  @override
  Widget build(BuildContext context) {
    final isActive = TickerMode.valuesOf(context).enabled;
    return LayoutBuilder(
      builder: (context, constraints) {
        if (!LibraryGalleryWall._hasUsableViewport(constraints)) {
          _pendingReflow = null;
          _layoutCorrection = null;
          return const SizedBox.shrink();
        }
        const horizontalPadding = LibraryGalleryWall._leadingHorizontalPadding;
        final availableWidth =
            constraints.maxWidth -
            LibraryGalleryWall._leadingHorizontalPadding -
            LibraryGalleryWall._trailingHorizontalPadding;
        if (!LibraryGalleryWall._hasUsableAvailableWidth(availableWidth)) {
          _pendingReflow = null;
          _layoutCorrection = null;
          return const SizedBox.shrink();
        }
        final snapshot = _snapshotFor(availableWidth, constraints.maxHeight);
        final canCompleteLayoutTransition = _prepareInitialPositionCorrection(
          snapshot,
          constraints.maxHeight,
        );
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
        widget.positionResolver?.update(
          queryId: widget.state.queryId,
          revision: widget.state.catalogRevision,
          resolve: (scrollOffset, viewportDimension) =>
              _positionAtOffset(snapshot, scrollOffset, viewportDimension),
        );
        final isPublishedLayout =
            (snapshot.availableWidth - availableWidth).abs() < 0.01;
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!mounted ||
              !isActive ||
              !TickerMode.getValuesNotifier(context).value.enabled) {
            return;
          }
          if (isPublishedLayout) {
            widget.onLayoutChanged(snapshot.metrics, virtualGeometry);
            _requestVisibleDetailRows(
              snapshot,
              direction: LibraryPreviewMovementDirection.idle,
            );
          }
        });
        final content = ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
          child: Stack(
            children: [
              LibraryGalleryInteraction(
                state: widget.state,
                scrollController: widget.scrollController,
                geometry: virtualGeometry,
                onLoadPrevious: widget.onLoadPrevious,
                onLoadNext: widget.controller.loadNextPage,
                onMetricsChanged: (metrics) {
                  if (isPublishedLayout) {
                    final position = _positionAtOffset(
                      snapshot,
                      metrics.pixels,
                      metrics.viewportDimension,
                    );
                    if (position != null) {
                      widget.onVisiblePositionChanged(position);
                    }
                    _requestVisibleDetailRows(
                      snapshot,
                      scrollMetrics: metrics,
                      direction: LibraryPreviewMovementDirection.idle,
                    );
                  }
                },
                onPositionChanged: (metrics, direction) {
                  final position = _positionAtOffset(
                    snapshot,
                    metrics.pixels,
                    metrics.viewportDimension,
                  );
                  if (position != null) {
                    widget.onVisiblePositionChanged(position);
                  }
                  _requestVisibleDetailRows(
                    snapshot,
                    scrollMetrics: metrics,
                    direction: direction,
                  );
                },
                child: KeyedSubtree(
                  key: const Key("library-photo-wall"),
                  child: CustomScrollView(
                    clipBehavior: Clip.none,
                    key: _LibraryGalleryScrollViewKey(widget.scrollController),
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
                          onLayoutCorrectionApplied: (generation) {
                            final transition = widget.layoutTransition;
                            if (canCompleteLayoutTransition &&
                                transition != null &&
                                generation ==
                                    (
                                      scope: "gallery-transition",
                                      value: transition.generation,
                                    )) {
                              widget.onLayoutTransitionApplied?.call(
                                transition.generation,
                              );
                            }
                          },
                          addSemanticIndexes: false,
                          itemBuilder: (context, index) {
                            return _buildEntry(
                              context,
                              snapshot,
                              index,
                              assetsByLocation,
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
                          icon: const Icon(Symbols.refresh_rounded),
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
                          icon: const Icon(Symbols.refresh_rounded),
                          label: const Text(LibraryStrings.retryLoading),
                        ),
                      ),
                    ),
                  ),
                ),
            ],
          ),
        );
        return LibraryGalleryLayoutMotion(
          queryIdentity: (
            widget.state.queryId,
            widget.state.catalogRevision,
            widget.layoutShape,
            widget.thumbnailSize,
          ),
          geometryIdentity: (snapshot.availableWidth, _publishedViewportExtent),
          scrollController: widget.scrollController,
          scrollOffsetCorrection: _layoutCorrection?.delta ?? 0,
          child: content,
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
          otherMinimumCellWidth: LibraryGalleryCardMetrics.minimumWidth,
          otherThumbnailSize: widget.thumbnailSize,
          otherLayoutShape: widget.layoutShape,
          otherSortKey: widget.state.query.sortKey,
        ) &&
        publishedViewportExtent != null &&
        (publishedViewportExtent - viewportExtent).abs() < 0.01) {
      return current;
    }
    if (current != null &&
        LibraryGalleryWall._hasUsableAvailableWidth(current.availableWidth) &&
        current.canReplaceGeometry(
          otherManifest: widget.manifest,
          otherThumbnailSize: widget.thumbnailSize,
          otherLayoutShape: widget.layoutShape,
          otherSortKey: widget.state.query.sortKey,
        )) {
      _scheduleResizeSnapshot(availableWidth, viewportExtent);
      return current;
    }
    _pendingReflow = null;
    _layoutCorrection = null;
    _publishedViewportExtent = viewportExtent;
    return _snapshot = LibraryGalleryLayoutSnapshot.build(
      minimumCellWidth: LibraryGalleryCardMetrics.minimumWidth,
      manifest: widget.manifest,
      availableWidth: availableWidth,
      thumbnailSize: widget.thumbnailSize,
      layoutShape: widget.layoutShape,
      sortKey: widget.state.query.sortKey,
    );
  }

  bool _prepareInitialPositionCorrection(
    LibraryGalleryLayoutSnapshot snapshot,
    double viewportExtent,
  ) {
    if (_didApplyInitialPosition) {
      return widget.layoutTransition != null;
    }
    final transition = widget.layoutTransition;
    if (transition != null && !identical(snapshot.manifest, widget.manifest)) {
      return true;
    }
    final position = transition == null
        ? widget.initialQueryWidePosition
        : transition.resolvePosition(
            current: widget.scrollController.hasClients
                ? widget.scrollController.position
                : null,
            resolver: widget.positionResolver,
            fallback: widget.initialQueryWidePosition,
          );
    if (position == null) {
      _didApplyInitialPosition = true;
      if (transition != null) {
        _layoutCorrection = LibraryExactExtentLayoutCorrection(
          generation: (
            scope: "gallery-transition",
            value: transition.generation,
          ),
          delta: 0,
        );
      }
      return transition != null;
    }
    if (position.queryId != snapshot.manifest.queryId ||
        position.revision != snapshot.manifest.revision ||
        snapshot.manifest.itemCount == 0) {
      _didApplyInitialPosition = true;
      return false;
    }
    final target = LibraryGalleryViewportAnchor(
      queryId: position.queryId,
      revision: position.revision,
      locationId: position.locationId,
      globalItemIndex: position.globalItemIndex,
      itemFraction: position.itemFraction,
      viewportFraction: position.viewportFraction,
      scrollEdge: transition?.edgeForScrollPosition(
        widget.scrollController.hasClients
            ? widget.scrollController.position
            : null,
      ),
    ).scrollOffsetFor(snapshot, viewportExtent);
    if (target == null) {
      if (transition == null) {
        _didApplyInitialPosition = true;
      }
      return false;
    }
    _didApplyInitialPosition = true;
    final delta =
        target -
        (widget.scrollController.hasClients
            ? widget.scrollController.position.pixels
            : widget.scrollController.initialScrollOffset);
    if (delta.abs() < 0.001) {
      _layoutCorrection = LibraryExactExtentLayoutCorrection(
        generation: transition == null
            ? ++_layoutCorrectionGeneration
            : (scope: "gallery-transition", value: transition.generation),
        delta: 0,
      );
      return transition != null;
    }
    _layoutCorrectionGeneration += 1;
    _layoutCorrection = LibraryExactExtentLayoutCorrection(
      generation: transition == null
          ? _layoutCorrectionGeneration
          : (scope: "gallery-transition", value: transition.generation),
      delta: delta,
    );
    return transition != null;
  }

  void _requestVisibleDetailRows(
    LibraryGalleryLayoutSnapshot snapshot, {
    ScrollMetrics? scrollMetrics,
    required LibraryPreviewMovementDirection direction,
  }) {
    if (widget.state.assets.isEmpty ||
        !TickerMode.getValuesNotifier(context).value.enabled) {
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
    final visibleRange = LibraryGalleryViewportFeedback.visibleRangeFor(
      layoutMetrics: layoutMetrics,
      scrollOffset: metrics.pixels,
      viewportDimension: metrics.viewportDimension,
      queryId: snapshot.manifest.queryId,
      revision: snapshot.manifest.revision,
    );
    if (visibleRange == null) {
      return;
    }
    widget.onVisibleRangeChanged?.call(visibleRange);
    final centerPosition = _positionAtOffset(
      snapshot,
      metrics.pixels,
      metrics.viewportDimension,
    );
    LibraryGalleryViewportFeedback.updatePreviewDemand(
      controller: widget.controller,
      state: widget.state,
      layoutMetrics: layoutMetrics,
      scrollOffset: metrics.pixels,
      viewportDimension: metrics.viewportDimension,
      direction: direction,
      anchorGlobalItemIndex:
          centerPosition?.globalItemIndex ?? visibleRange.startGlobalItemIndex,
      previewEdgeFor: (_, itemIndex) => libraryPreviewDecodeWidth(
        snapshot.displayExtentForItemIndex(itemIndex),
        MediaQuery.devicePixelRatioOf(context),
      ),
    );
    widget.controller.ensureVisibleRange(
      startItemOffset: visibleRange.startGlobalItemIndex,
      endItemOffsetExclusive: visibleRange.endGlobalItemIndexExclusive,
    );
  }

  void _scheduleResizeSnapshot(double availableWidth, double viewportExtent) {
    if (!LibraryGalleryWall._hasUsableAvailableWidth(availableWidth) ||
        !viewportExtent.isFinite ||
        viewportExtent <= 0) {
      return;
    }
    final current = _snapshot;
    final publishedViewportExtent = _publishedViewportExtent;
    final isWidthOnlyResize =
        current != null &&
        current.matchesInputs(
          otherManifest: widget.manifest,
          otherMinimumCellWidth: LibraryGalleryCardMetrics.minimumWidth,
          otherThumbnailSize: widget.thumbnailSize,
          otherLayoutShape: widget.layoutShape,
          otherSortKey: widget.state.query.sortKey,
        ) &&
        (current.availableWidth - availableWidth).abs() >= 0.01 &&
        publishedViewportExtent != null &&
        (publishedViewportExtent - viewportExtent).abs() < 0.01;
    if (current != null && isWidthOnlyResize && widget.isSidebarResizing) {
      _sidebarResizeAnchor ??= _captureViewportAnchor(
        current,
        viewportFraction: 0.0,
      );
    }
    final position = widget.scrollController.hasClients
        ? widget.scrollController.position
        : null;
    final transitionAnchor = widget.layoutTransition?.anchorForScrollPosition(
      position,
    );
    final anchor =
        transitionAnchor ??
        (current == null
            ? null
            : isWidthOnlyResize && widget.isSidebarResizing
            ? _sidebarResizeAnchor
            : _captureViewportAnchor(current, viewportFraction: 0.5));
    final pending = _pendingReflow;
    if (pending == null) {
      _pendingReflow = LibraryGalleryPendingReflow(
        availableWidth: availableWidth,
        viewportExtent: viewportExtent,
        anchor: anchor,
        position: position,
        transitionGeneration: widget.layoutTransition?.generation,
      );
    } else {
      pending.updateViewport(availableWidth, viewportExtent);
    }
    if (_isResizeFrameScheduled) {
      return;
    }
    _isResizeFrameScheduled = true;
    SchedulerBinding.instance.scheduleFrameCallback((_) {
      _isResizeFrameScheduled = false;
      if (!mounted) {
        return;
      }
      final pending = _pendingReflow;
      _pendingReflow = null;
      if (pending == null) {
        return;
      }
      final pendingWidth = pending.availableWidth;
      final pendingViewportExtent = pending.viewportExtent;
      final current = _snapshot;
      final publishedViewportExtent = _publishedViewportExtent;
      if (current == null ||
          (current.matches(
                otherManifest: widget.manifest,
                otherAvailableWidth: pendingWidth,
                otherMinimumCellWidth: LibraryGalleryCardMetrics.minimumWidth,
                otherThumbnailSize: widget.thumbnailSize,
                otherLayoutShape: widget.layoutShape,
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
            otherMinimumCellWidth: LibraryGalleryCardMetrics.minimumWidth,
            otherThumbnailSize: widget.thumbnailSize,
            otherLayoutShape: widget.layoutShape,
            otherSortKey: widget.state.query.sortKey,
          )
          ? current
          : LibraryGalleryLayoutSnapshot.build(
              minimumCellWidth: LibraryGalleryCardMetrics.minimumWidth,
              manifest: widget.manifest,
              availableWidth: pendingWidth,
              thumbnailSize: widget.thumbnailSize,
              layoutShape: widget.layoutShape,
              sortKey: widget.state.query.sortKey,
            );
      final position = widget.scrollController.hasClients
          ? widget.scrollController.position
          : null;
      final transition = widget.layoutTransition;
      final pendingAnchor = pending.resolveAnchor(
        snapshot: current,
        position: position,
        transitionGeneration: transition?.generation,
        transitionAnchor: transition?.anchorForScrollPosition(position),
      );
      final target = pendingAnchor?.scrollOffsetFor(
        replacement,
        pendingViewportExtent,
      );
      final delta = target == null || position == null
          ? null
          : target - position.pixels;
      LibraryExactExtentLayoutCorrection? publishedCorrection;
      setState(() {
        _snapshot = replacement;
        _publishedViewportExtent = pendingViewportExtent;
        if (widget.isSidebarResizing) {
          _sidebarResizeAnchor = pendingAnchor;
        }
        if (delta == null || (delta.abs() < 0.001 && transition == null)) {
          _layoutCorrection = null;
        } else {
          _didApplyInitialPosition = true;
          _layoutCorrectionGeneration += 1;
          _layoutCorrection = LibraryExactExtentLayoutCorrection(
            generation: transition == null
                ? _layoutCorrectionGeneration
                : (scope: "gallery-transition", value: transition.generation),
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

  LibraryGalleryViewportAnchor? _captureViewportAnchor(
    LibraryGalleryLayoutSnapshot snapshot, {
    required double viewportFraction,
  }) {
    if (!widget.scrollController.hasClients) {
      return null;
    }
    final position = widget.scrollController.position;
    if (!position.hasViewportDimension) {
      return null;
    }
    return LibraryGalleryViewportAnchor.capture(
      snapshot,
      scrollOffset: position.pixels,
      viewportExtent: position.viewportDimension,
      viewportFraction: viewportFraction,
    );
  }

  Widget _buildEntry(
    BuildContext context,
    LibraryGalleryLayoutSnapshot snapshot,
    int entryIndex,
    Map<String, LibraryAsset> assetsByLocation,
  ) {
    final entry = snapshot.entries[entryIndex];
    final offset = 18 + snapshot.entryStartOffsets[entryIndex];
    if (entry.headerLabel case final label?) {
      return LibraryGalleryMotionHeader(
        label: label,
        dateKey: entry.dateKey,
        rect: Rect.fromLTWH(0, offset, snapshot.availableWidth, entry.extent),
      );
    }
    if (!entry.isPhotoRow) {
      return SizedBox(height: entry.extent);
    }
    return LibraryGalleryMotionRow(
      extent: entry.extent,
      width: snapshot.availableWidth,
      height: entry.rowHeight,
      offset: offset,
      cellWidths: entry.cellWidths,
      identityAt: (index) =>
          snapshot.manifest.locationIdAt(entry.startItemIndex + index),
      builder: (index, size) => _buildCell(
        snapshot,
        entry.startItemIndex + index,
        size.width,
        size.height,
        assetsByLocation,
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
      return _LibraryGalleryUnloadedSlot(
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
      isSelected: widget.selection.contains(asset.assetId),
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
    final anchor = LibraryGalleryViewportAnchor.capture(
      snapshot,
      scrollOffset: scrollOffset,
      viewportExtent: viewportDimension,
      viewportFraction: 0.5,
    );
    if (anchor == null) {
      return null;
    }
    return LibraryGalleryVisiblePosition(
      queryId: anchor.queryId,
      revision: anchor.revision,
      monthKey: anchor.monthKey,
      locationId: anchor.locationId,
      globalItemIndex: anchor.globalItemIndex,
      itemFraction: anchor.itemFraction,
      viewportFraction: anchor.viewportFraction,
    );
  }
}

class _LibraryGalleryUnloadedSlot extends StatelessWidget {
  const _LibraryGalleryUnloadedSlot({
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
      child: IgnorePointer(
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: Theme.of(context).colorScheme.surfaceContainerHighest,
            borderRadius: BorderRadius.circular(10),
          ),
        ),
      ),
    );
  }
}
