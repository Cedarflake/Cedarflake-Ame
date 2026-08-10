import "dart:math" as math;

import "../../domain/library_models.dart";
import "../gallery_view_options.dart";
import "library_gallery_layout.dart";
import "library_timeline_projection.dart";

class LibraryVirtualGalleryGeometry {
  const LibraryVirtualGalleryGeometry({
    required this.totalContentExtent,
    required this.viewportExtent,
    required this.leadingExtent,
    required this.loadedContentExtent,
    required this.trailingExtent,
    required this.windowStartItemOffset,
    required this.loadedItemCount,
    required this.totalItemCount,
    required this.queryId,
  });

  factory LibraryVirtualGalleryGeometry.calculate({
    required LibraryTimeline? timeline,
    required double availableWidth,
    required double viewportExtent,
    required GalleryLayoutShape layoutShape,
    required GalleryThumbnailSize thumbnailSize,
    required LibraryGallerySortKey sortKey,
    required double loadedContentExtent,
    required int windowStartItemOffset,
    required int loadedItemCount,
    required String queryId,
  }) {
    final totalItemCount = timeline?.totalItems ?? loadedItemCount;
    final hasUnloadedItems =
        timeline != null && totalItemCount > loadedItemCount;
    if (!hasUnloadedItems || availableWidth <= 0 || viewportExtent <= 0) {
      return LibraryVirtualGalleryGeometry(
        totalContentExtent: loadedContentExtent,
        viewportExtent: viewportExtent,
        leadingExtent: 0,
        loadedContentExtent: loadedContentExtent,
        trailingExtent: 0,
        windowStartItemOffset: windowStartItemOffset,
        loadedItemCount: loadedItemCount,
        totalItemCount: totalItemCount,
        queryId: queryId,
      );
    }

    final estimatedExtent = _estimateCompleteExtent(
      timeline: timeline,
      availableWidth: availableWidth,
      layoutShape: layoutShape,
      thumbnailSize: thumbnailSize,
      sortKey: sortKey,
    );
    final totalContentExtent = math.max(estimatedExtent, viewportExtent * 3);
    final maximumScrollExtent = math.max(
      0.0,
      totalContentExtent - viewportExtent,
    );
    final projection = LibraryTimelineProjection(
      timeline: timeline,
      useAspectRatioWeight: layoutShape == GalleryLayoutShape.equalHeight,
    );
    final startValue = projection.valueForGlobalItemOffset(
      windowStartItemOffset.toDouble(),
    );
    final maximumLeadingExtent = math.max(
      0.0,
      totalContentExtent - loadedContentExtent,
    );
    final leadingExtent = (startValue * maximumScrollExtent)
        .clamp(0.0, maximumLeadingExtent)
        .toDouble();
    final trailingExtent = math.max(
      0.0,
      totalContentExtent - leadingExtent - loadedContentExtent,
    );
    return LibraryVirtualGalleryGeometry(
      totalContentExtent: totalContentExtent,
      viewportExtent: viewportExtent,
      leadingExtent: leadingExtent,
      loadedContentExtent: loadedContentExtent,
      trailingExtent: trailingExtent,
      windowStartItemOffset: windowStartItemOffset,
      loadedItemCount: loadedItemCount,
      totalItemCount: totalItemCount,
      queryId: queryId,
    );
  }

  final double totalContentExtent;
  final double viewportExtent;
  final double leadingExtent;
  final double loadedContentExtent;
  final double trailingExtent;
  final int windowStartItemOffset;
  final int loadedItemCount;
  final int totalItemCount;
  final String queryId;

  bool get isVirtualized => leadingExtent > 0 || trailingExtent > 0;

  double get maximumScrollExtent =>
      math.max(0.0, totalContentExtent - viewportExtent);

  double get loadedEndExtent => leadingExtent + loadedContentExtent;

  bool containsGlobalItemOffset(double itemOffset) {
    return itemOffset >= windowStartItemOffset &&
        itemOffset < windowStartItemOffset + loadedItemCount;
  }

  bool containsScrollOffset(double scrollOffset) {
    final visibleOffset = scrollOffset + (viewportExtent * 0.5);
    return visibleOffset >= leadingExtent && visibleOffset < loadedEndExtent;
  }

  double scrollOffsetForValue(double value) {
    return value.clamp(0.0, 1.0).toDouble() * maximumScrollExtent;
  }

  double valueForScrollOffset(double scrollOffset) {
    if (maximumScrollExtent <= 0) {
      return 0;
    }
    return (scrollOffset / maximumScrollExtent).clamp(0.0, 1.0).toDouble();
  }

  bool hasSameGeometry(LibraryVirtualGalleryGeometry other) {
    return (totalContentExtent - other.totalContentExtent).abs() < 0.01 &&
        (viewportExtent - other.viewportExtent).abs() < 0.01 &&
        (leadingExtent - other.leadingExtent).abs() < 0.01 &&
        (loadedContentExtent - other.loadedContentExtent).abs() < 0.01 &&
        (trailingExtent - other.trailingExtent).abs() < 0.01 &&
        windowStartItemOffset == other.windowStartItemOffset &&
        loadedItemCount == other.loadedItemCount &&
        totalItemCount == other.totalItemCount &&
        queryId == other.queryId;
  }

  static double _estimateCompleteExtent({
    required LibraryTimeline timeline,
    required double availableWidth,
    required GalleryLayoutShape layoutShape,
    required GalleryThumbnailSize thumbnailSize,
    required LibraryGallerySortKey sortKey,
  }) {
    final spacing = LibraryGalleryLayoutEntry.spacing;
    final targetExtent = thumbnailSize.targetExtent;
    final maximumDateGroups = sortKey == LibraryGallerySortKey.fileName
        ? 0
        : timeline.buckets.fold(
            0,
            (sum, bucket) => sum + math.min(bucket.itemCount, 31),
          );
    final estimatedPhotoExtent = switch (layoutShape) {
      GalleryLayoutShape.square => () {
        final columnCount =
            ((availableWidth + spacing) / (targetExtent + spacing))
                .floor()
                .clamp(1, math.max(1, timeline.totalItems));
        final tileExtent =
            (availableWidth - spacing * (columnCount - 1)) / columnCount;
        final rowCount = sortKey == LibraryGallerySortKey.fileName
            ? (timeline.totalItems / columnCount).ceil()
            : timeline.buckets.fold(0, (sum, bucket) {
                final groupCount = math.min(bucket.itemCount, 31);
                final remainingItems = math.max(
                  0,
                  bucket.itemCount - groupCount,
                );
                return sum + groupCount + (remainingItems / columnCount).ceil();
              });
        return rowCount * (tileExtent + spacing);
      }(),
      GalleryLayoutShape.equalHeight => () {
        final aspectRatioSum = timeline.buckets.fold(
          0.0,
          (sum, bucket) => sum + bucket.aspectRatioSum,
        );
        final effectiveAspectRatioSum = aspectRatioSum > 0
            ? aspectRatioSum
            : timeline.totalItems.toDouble();
        final estimatedRowCount = math.max(
          1,
          ((effectiveAspectRatioSum * targetExtent / availableWidth) * 1.25)
                  .ceil() +
              maximumDateGroups,
        );
        return estimatedRowCount * (targetExtent + spacing);
      }(),
    };
    final groupExtent =
        maximumDateGroups *
        (LibraryGalleryLayoutEntry.headerExtent +
            LibraryGalleryLayoutEntry.groupGap);
    return 18 + estimatedPhotoExtent + groupExtent + 72;
  }
}
