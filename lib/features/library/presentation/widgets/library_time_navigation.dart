import "package:flutter/material.dart";

import "../../domain/library_models.dart";
import "../gallery_view_options.dart";
import "annotated_time_rail.dart";
import "library_gallery_layout.dart";
import "library_timeline_projection.dart";

typedef LibraryTimelineSeekCallback =
    Future<bool> Function(LibraryTimeBucket bucket, int itemOffset);

class LibraryTimeNavigation extends StatefulWidget {
  const LibraryTimeNavigation({
    required this.isLoading,
    required this.scrollController,
    required this.layoutMetrics,
    required this.timeline,
    required this.layoutShape,
    required this.windowStartItemOffset,
    required this.loadedItemCount,
    required this.onSeek,
    super.key,
  });

  final bool isLoading;
  final ScrollController scrollController;
  final LibraryGalleryLayoutMetrics? layoutMetrics;
  final LibraryTimeline? timeline;
  final GalleryLayoutShape layoutShape;
  final int windowStartItemOffset;
  final int loadedItemCount;
  final LibraryTimelineSeekCallback onSeek;

  @override
  State<LibraryTimeNavigation> createState() => _LibraryTimeNavigationState();
}

class _LibraryTimeNavigationState extends State<LibraryTimeNavigation> {
  double? _interactiveValue;
  bool _isSeeking = false;

  @override
  void didUpdateWidget(covariant LibraryTimeNavigation oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.timeline?.revision != widget.timeline?.revision ||
        oldWidget.timeline?.queryId != widget.timeline?.queryId) {
      _interactiveValue = null;
      _isSeeking = false;
    }
  }

  @override
  Widget build(BuildContext context) {
    final timeline = widget.timeline;
    if (widget.isLoading && timeline == null) {
      return const SizedBox(
        width: 80,
        child: Center(
          child: SizedBox.square(
            dimension: 24,
            child: CircularProgressIndicator(strokeWidth: 3),
          ),
        ),
      );
    }
    final metrics = widget.layoutMetrics;
    if (timeline == null ||
        timeline.buckets.isEmpty ||
        metrics == null ||
        metrics.dateAnchors.isEmpty) {
      return const SizedBox.shrink();
    }
    final globalProjection = LibraryTimelineProjection(
      timeline: timeline,
      useAspectRatioWeight:
          widget.layoutShape == GalleryLayoutShape.equalHeight,
    );
    return AnimatedBuilder(
      animation: widget.scrollController,
      builder: (context, child) {
        final position = widget.scrollController.hasClients
            ? widget.scrollController.position
            : null;
        final derivedValue = _valueFromGallery(
          globalProjection,
          position,
          metrics,
        );
        return AnnotatedTimeRail(
          key: const Key("library-time-rail"),
          value: _interactiveValue ?? derivedValue,
          maximumScrollOffset: globalProjection.maximumOffset,
          buckets: globalProjection.railBuckets,
          projection: globalProjection.projection,
          onChanged: (value) =>
              _handleChanged(globalProjection, position, metrics, value),
          onChangeEnd: (value) =>
              _commit(globalProjection, position, metrics, value),
          onStep: (direction) => _moveOneRow(metrics, direction),
        );
      },
    );
  }

  double _valueFromGallery(
    LibraryTimelineProjection globalProjection,
    ScrollPosition? position,
    LibraryGalleryLayoutMetrics metrics,
  ) {
    if (position == null ||
        !position.hasContentDimensions ||
        widget.loadedItemCount <= 1) {
      return globalProjection.valueForGlobalItemOffset(
        widget.windowStartItemOffset.toDouble(),
      );
    }
    final localItemOffset = metrics.itemIndexForScrollOffset(position.pixels);
    final globalItemOffset = widget.windowStartItemOffset + localItemOffset;
    return globalProjection.valueForGlobalItemOffset(
      globalItemOffset.toDouble(),
    );
  }

  void _handleChanged(
    LibraryTimelineProjection globalProjection,
    ScrollPosition? position,
    LibraryGalleryLayoutMetrics metrics,
    double value,
  ) {
    if (_isSeeking) {
      return;
    }
    setState(() => _interactiveValue = value);
    _jumpWithinLoadedWindow(globalProjection, position, metrics, value);
  }

  Future<void> _commit(
    LibraryTimelineProjection globalProjection,
    ScrollPosition? position,
    LibraryGalleryLayoutMetrics metrics,
    double value,
  ) async {
    if (_isSeeking) {
      return;
    }
    final target = globalProjection.targetForValue(value);
    final loadedEnd = widget.windowStartItemOffset + widget.loadedItemCount - 1;
    if (target.globalItemOffset >= widget.windowStartItemOffset &&
        target.globalItemOffset <= loadedEnd &&
        _jumpWithinLoadedWindow(globalProjection, position, metrics, value)) {
      if (mounted) {
        setState(() => _interactiveValue = null);
      }
      return;
    }
    setState(() => _isSeeking = true);
    final didSeek = await widget.onSeek(target.bucket, target.itemOffset);
    if (!mounted) {
      return;
    }
    if (didSeek) {
      WidgetsBinding.instance.scheduleFrame();
      await WidgetsBinding.instance.endOfFrame;
      if (mounted && widget.scrollController.hasClients) {
        widget.scrollController.jumpTo(
          widget.scrollController.position.minScrollExtent,
        );
      }
    }
    if (mounted) {
      setState(() {
        _isSeeking = false;
        _interactiveValue = null;
      });
    }
  }

  bool _jumpWithinLoadedWindow(
    LibraryTimelineProjection globalProjection,
    ScrollPosition? position,
    LibraryGalleryLayoutMetrics metrics,
    double value,
  ) {
    if (position == null ||
        !position.hasContentDimensions ||
        widget.loadedItemCount <= 0) {
      return false;
    }
    final targetGlobalOffset = globalProjection.globalItemOffsetForValue(value);
    final loadedEnd = widget.windowStartItemOffset + widget.loadedItemCount;
    if (targetGlobalOffset < widget.windowStartItemOffset ||
        targetGlobalOffset >= loadedEnd) {
      return false;
    }
    final localItemIndex = (targetGlobalOffset - widget.windowStartItemOffset)
        .floor()
        .clamp(0, widget.loadedItemCount - 1)
        .toInt();
    final targetPixels = metrics.offsetForItemIndex(localItemIndex);
    if (targetPixels == null) {
      return false;
    }
    position.jumpTo(
      targetPixels
          .clamp(position.minScrollExtent, position.maxScrollExtent)
          .toDouble(),
    );
    return true;
  }

  void _moveOneRow(LibraryGalleryLayoutMetrics metrics, int direction) {
    if (!widget.scrollController.hasClients || metrics.photoRowHeight <= 0) {
      return;
    }
    final position = widget.scrollController.position;
    if (!position.hasContentDimensions) {
      return;
    }
    final target =
        position.pixels +
        (direction *
            (metrics.photoRowHeight + LibraryGalleryLayoutEntry.spacing));
    widget.scrollController.animateTo(
      target
          .clamp(position.minScrollExtent, position.maxScrollExtent)
          .toDouble(),
      duration: const Duration(milliseconds: 140),
      curve: Curves.easeOutCubic,
    );
  }
}
