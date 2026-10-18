import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "../gallery_view_options.dart";
import "library_gallery_layout.dart";
import "library_virtual_gallery_geometry.dart";

class LibraryGalleryVisibleRange {
  const LibraryGalleryVisibleRange({
    required this.queryId,
    required this.revision,
    required this.startGlobalItemIndex,
    required this.endGlobalItemIndexExclusive,
  });

  final String queryId;
  final BigInt revision;
  final int startGlobalItemIndex;
  final int endGlobalItemIndexExclusive;

  bool contains({
    required String queryId,
    required BigInt revision,
    required int globalItemIndex,
  }) {
    return this.queryId == queryId &&
        this.revision == revision &&
        containsGlobalItemIndex(globalItemIndex);
  }

  bool containsGlobalItemIndex(int globalItemIndex) {
    return globalItemIndex >= startGlobalItemIndex &&
        globalItemIndex < endGlobalItemIndexExclusive;
  }

  bool matches(LibraryGalleryVisibleRange other) {
    return queryId == other.queryId &&
        revision == other.revision &&
        startGlobalItemIndex == other.startGlobalItemIndex &&
        endGlobalItemIndexExclusive == other.endGlobalItemIndexExclusive;
  }
}

enum LibraryPreviewMovementDirection { backward, idle, forward }

class LibraryGalleryViewportFeedback {
  static const double _directionalPreviewViewportCount = 1.5;
  static const double _guardPreviewViewportCount = 0.75;

  static void updatePreviewDemand({
    required LibraryController controller,
    required LibraryState state,
    required LibraryGalleryLayoutMetrics layoutMetrics,
    required double scrollOffset,
    required double viewportDimension,
    required LibraryPreviewMovementDirection direction,
    required int anchorGlobalItemIndex,
    required int Function(LibraryAsset asset, int globalItemIndex)
    previewEdgeFor,
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

    final previewEdges = <String, int>{};

    List<LibraryAsset> assetsInRange(int start, int end) {
      final loadedStart = state.windowStartItemOffset;
      final loadedEnd = loadedStart + state.assets.length;
      final boundedStart = start.clamp(loadedStart, loadedEnd).toInt();
      final boundedEnd = end.clamp(boundedStart, loadedEnd).toInt();
      if (boundedEnd <= boundedStart) {
        return const [];
      }
      final itemIndices =
          [
            for (
              var itemIndex = boundedStart;
              itemIndex < boundedEnd;
              itemIndex += 1
            )
              itemIndex,
          ]..sort((first, second) {
            final distanceComparison = (first - anchorGlobalItemIndex)
                .abs()
                .compareTo((second - anchorGlobalItemIndex).abs());
            return distanceComparison != 0
                ? distanceComparison
                : first.compareTo(second);
          });
      final assets = <LibraryAsset>[];
      for (final itemIndex in itemIndices) {
        final asset = state.assets[itemIndex - loadedStart];
        previewEdges[asset.locationId] = previewEdgeFor(asset, itemIndex);
        assets.add(asset);
      }
      return assets;
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
      LibraryPreviewMovementDirection.backward => assetsInRange(
        backwardDirectionalStart,
        visibleStart,
      ),
      LibraryPreviewMovementDirection.idle ||
      LibraryPreviewMovementDirection.forward => assetsInRange(
        visibleEnd,
        forwardDirectionalEnd,
      ),
    };
    final guard = switch (direction) {
      LibraryPreviewMovementDirection.backward => assetsInRange(
        visibleEnd,
        forwardGuardEnd,
      ),
      LibraryPreviewMovementDirection.forward => assetsInRange(
        backwardGuardStart,
        visibleStart,
      ),
      LibraryPreviewMovementDirection.idle => [
        ...assetsInRange(backwardGuardStart, visibleStart),
        ...assetsInRange(visibleEnd, forwardGuardEnd),
      ],
    };
    controller.updateGalleryPreviewDemand(
      visible: assetsInRange(visibleStart, visibleEnd),
      nearDirection: nearDirection,
      guard: guard,
      previewEdges: previewEdges,
    );
  }

  static LibraryGalleryVisibleRange? visibleRangeFor({
    required LibraryGalleryLayoutMetrics layoutMetrics,
    required double scrollOffset,
    required double viewportDimension,
    required String queryId,
    required BigInt? revision,
  }) {
    if (revision == null ||
        queryId.isEmpty ||
        !viewportDimension.isFinite ||
        viewportDimension <= 0) {
      return null;
    }
    final visibleStart = scrollOffset
        .clamp(0.0, layoutMetrics.contentExtent)
        .toDouble();
    final visibleEnd = (scrollOffset + viewportDimension)
        .clamp(visibleStart, layoutMetrics.contentExtent)
        .toDouble();
    final firstVisibleItem =
        layoutMetrics.itemIndexBase +
        layoutMetrics.itemIndexForScrollOffset(visibleStart);
    final lastVisibleItem =
        layoutMetrics.itemIndexBase +
        layoutMetrics.itemIndexForScrollOffset(visibleEnd);
    final firstVisibleRowStart = layoutMetrics.rowStartGlobalItemIndex(
      firstVisibleItem,
    );
    final lastVisibleRowEnd = layoutMetrics.rowEndGlobalItemIndexExclusive(
      lastVisibleItem,
    );
    if (firstVisibleRowStart == null || lastVisibleRowEnd == null) {
      return null;
    }
    return LibraryGalleryVisibleRange(
      queryId: queryId,
      revision: revision,
      startGlobalItemIndex: firstVisibleRowStart,
      endGlobalItemIndexExclusive: lastVisibleRowEnd,
    );
  }

  static LibraryGalleryVisibleRange? estimatedSquareVisibleRangeFor({
    required LibraryVirtualGalleryGeometry virtualGeometry,
    required double scrollOffset,
    required double viewportDimension,
    required double availableWidth,
    required GalleryThumbnailSize thumbnailSize,
    required String queryId,
    required BigInt? revision,
  }) {
    final totalItems = virtualGeometry.totalItemCount;
    if (revision == null ||
        queryId.isEmpty ||
        totalItems <= 0 ||
        !scrollOffset.isFinite ||
        !viewportDimension.isFinite ||
        viewportDimension <= 0 ||
        availableWidth <= 0) {
      return null;
    }
    final spacing = LibraryGalleryLayoutEntry.spacing;
    final columnCount =
        ((availableWidth + spacing) / (thumbnailSize.targetExtent + spacing))
            .floor()
            .clamp(1, totalItems)
            .toInt();
    final tileExtent =
        (availableWidth - spacing * (columnCount - 1)) / columnCount;
    final minimumSpan =
        columnCount * ((viewportDimension / (tileExtent + spacing)).ceil() + 2);
    final startValue = virtualGeometry.valueForScrollOffset(scrollOffset);
    final endValue = virtualGeometry.valueForScrollOffset(
      scrollOffset + viewportDimension,
    );
    var start = (startValue * totalItems)
        .floor()
        .clamp(0, totalItems - 1)
        .toInt();
    var end = (endValue * totalItems)
        .ceil()
        .clamp(start + 1, totalItems)
        .toInt();
    if (end - start < minimumSpan) {
      final center = ((start + end) / 2).round();
      start = (center - minimumSpan ~/ 2).clamp(0, totalItems - 1).toInt();
      end = (start + minimumSpan).clamp(start + 1, totalItems).toInt();
      if (end - start < minimumSpan) {
        start = (end - minimumSpan).clamp(0, totalItems - 1).toInt();
      }
    }
    start -= start % columnCount;
    end = (((end + columnCount - 1) ~/ columnCount) * columnCount)
        .clamp(start + 1, totalItems)
        .toInt();
    return LibraryGalleryVisibleRange(
      queryId: queryId,
      revision: revision,
      startGlobalItemIndex: start,
      endGlobalItemIndexExclusive: end,
    );
  }
}
