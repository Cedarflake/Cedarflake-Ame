import "dart:async";

import "package:flutter/gestures.dart";
import "package:flutter/material.dart";

import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "../gallery_selection.dart";
import "../gallery_view_options.dart";
import "../library_strings.dart";
import "library_gallery_layout.dart";
import "library_photo_tile.dart";

class LibraryGalleryVisiblePosition {
  const LibraryGalleryVisiblePosition({
    required this.monthKey,
    required this.locationId,
  });

  final String? monthKey;
  final String? locationId;
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
    required this.onOpen,
    required this.onToggleSelection,
    required this.onViewInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    required this.onVisiblePositionChanged,
    required this.onLoadPrevious,
    required this.onLayoutChanged,
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
  final ValueChanged<LibraryGalleryLayoutMetrics> onLayoutChanged;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        const horizontalPadding = 24.0;
        final entries = LibraryGalleryLayoutEntry.build(
          assets: state.assets,
          availableWidth: constraints.maxWidth - horizontalPadding - 16,
          layoutShape: layoutShape,
          thumbnailSize: thumbnailSize,
          sortKey: state.query.sortKey,
        );
        final layoutMetrics = LibraryGalleryLayoutMetrics.fromEntries(
          entries,
          topPadding: 18,
          bottomPadding: 72,
        );
        WidgetsBinding.instance.addPostFrameCallback((_) {
          onLayoutChanged(layoutMetrics);
        });
        return ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
          child: Stack(
            children: [
              Listener(
                onPointerSignal: (event) {
                  if (event is PointerScrollEvent &&
                      event.scrollDelta.dy < 0 &&
                      scrollController.hasClients &&
                      scrollController.position.extentBefore < 900 &&
                      state.hasPreviousAssets &&
                      !state.isLoadingPage &&
                      !state.isLoadingPreviousPage &&
                      state.previousPageErrorMessage == null) {
                    unawaited(onLoadPrevious());
                  }
                },
                child: NotificationListener<ScrollNotification>(
                  onNotification: (notification) {
                    if (notification is ScrollUpdateNotification ||
                        notification is ScrollEndNotification) {
                      final position = _positionAtOffset(
                        entries,
                        notification.metrics.pixels,
                        notification.metrics.viewportDimension,
                      );
                      if (position != null) {
                        onVisiblePositionChanged(position);
                      }
                    }
                    if (notification.metrics.extentBefore < 900 &&
                        state.hasPreviousAssets &&
                        !state.isLoadingPage &&
                        !state.isLoadingPreviousPage &&
                        state.previousPageErrorMessage == null) {
                      unawaited(onLoadPrevious());
                    }
                    if (notification.metrics.extentAfter < 900 &&
                        state.hasMoreAssets &&
                        !state.isLoadingPage &&
                        !state.isLoadingPreviousPage &&
                        state.pageErrorMessage == null) {
                      unawaited(controller.loadNextPage());
                    }
                    return false;
                  },
                  child: CustomScrollView(
                    key: const Key("library-photo-wall"),
                    controller: scrollController,
                    slivers: [
                      SliverPadding(
                        padding: const EdgeInsets.fromLTRB(
                          horizontalPadding,
                          18,
                          16,
                          72,
                        ),
                        sliver: SliverVariedExtentList.builder(
                          itemCount: entries.length,
                          itemExtentBuilder: (index, _) =>
                              entries[index].extent,
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
                      if (state.isLoadingPage || state.pageErrorMessage != null)
                        SliverToBoxAdapter(
                          child: SizedBox(
                            height: 72,
                            child: state.isLoadingPage
                                ? const Center(
                                    child: SizedBox.square(
                                      dimension: 28,
                                      child: CircularProgressIndicator(
                                        strokeWidth: 3,
                                      ),
                                    ),
                                  )
                                : Center(
                                    child: OutlinedButton.icon(
                                      key: const Key(
                                        "library-load-more-button",
                                      ),
                                      onPressed: controller.loadNextPage,
                                      icon: const Icon(Icons.refresh),
                                      label: const Text(
                                        LibraryStrings.retryLoading,
                                      ),
                                    ),
                                  ),
                          ),
                        ),
                    ],
                  ),
                ),
              ),
              if (state.isLoadingPreviousPage ||
                  state.previousPageErrorMessage != null)
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
                        child: state.isLoadingPreviousPage
                            ? const SizedBox.square(
                                key: Key("library-load-previous-progress"),
                                dimension: 22,
                                child: CircularProgressIndicator(
                                  strokeWidth: 3,
                                ),
                              )
                            : TextButton.icon(
                                key: const Key("library-load-previous-button"),
                                onPressed: onLoadPrevious,
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

  static LibraryGalleryVisiblePosition? _positionAtOffset(
    List<LibraryGalleryLayoutEntry> entries,
    double scrollOffset,
    double viewportDimension,
  ) {
    if (entries.isEmpty) {
      return null;
    }
    final target = (scrollOffset + (viewportDimension * 0.5) - 18).clamp(
      0.0,
      double.infinity,
    );
    var activeEntry = entries.first;
    var runningOffset = 0.0;
    for (final entry in entries) {
      if (runningOffset > target) {
        break;
      }
      activeEntry = entry;
      runningOffset += entry.extent;
    }
    return LibraryGalleryVisiblePosition(
      monthKey: activeEntry.monthKey,
      locationId: activeEntry.firstLocationId,
    );
  }
}
