import "dart:async";

import "package:flutter/gestures.dart";
import "package:flutter/widgets.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_page_operation.dart";
import "../../domain/library_state.dart";
import "../coordination/library_gallery_page_demand.dart";
import "library_gallery_viewport_feedback.dart";
import "library_virtual_gallery_geometry.dart";

class LibraryGalleryInteraction extends StatefulWidget {
  const LibraryGalleryInteraction({
    required this.state,
    required this.scrollController,
    required this.geometry,
    required this.onLoadPrevious,
    required this.onLoadNext,
    required this.onMetricsChanged,
    required this.onPositionChanged,
    required this.child,
    super.key,
  });

  final LibraryState state;
  final ScrollController scrollController;
  final LibraryVirtualGalleryGeometry geometry;
  final Future<void> Function() onLoadPrevious;
  final Future<void> Function() onLoadNext;
  final ValueChanged<ScrollMetrics> onMetricsChanged;
  final void Function(
    ScrollMetrics metrics,
    LibraryPreviewMovementDirection direction,
  )
  onPositionChanged;
  final Widget child;

  @override
  State<LibraryGalleryInteraction> createState() =>
      _LibraryGalleryInteractionState();
}

class _LibraryGalleryInteractionState extends State<LibraryGalleryInteraction> {
  final _pageDemand = LibraryGalleryPageDemand();
  Timer? _pageCheck;

  @override
  void dispose() {
    _pageCheck?.cancel();
    super.dispose();
  }

  @override
  void didUpdateWidget(covariant LibraryGalleryInteraction oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.scrollController != widget.scrollController ||
        oldWidget.state.query != widget.state.query) {
      _pageDemand.clear();
    }
    _schedulePageRequest();
  }

  bool _isNearLoadedStart(ScrollMetrics metrics) {
    return metrics.pixels >=
            widget.geometry.leadingExtent - metrics.viewportDimension &&
        metrics.pixels <= widget.geometry.leadingExtent + 900;
  }

  bool _isNearLoadedEnd(ScrollMetrics metrics) {
    final visibleBottom = metrics.pixels + metrics.viewportDimension;
    return visibleBottom >= widget.geometry.loadedEndExtent - 900 &&
        metrics.pixels <= widget.geometry.loadedEndExtent;
  }

  void _onPointerSignal(PointerSignalEvent event) {
    if (event is! PointerScrollEvent || !widget.scrollController.hasClients) {
      return;
    }
    _pageDemand.scroll(widget.state.query, event.scrollDelta.dy);
    _schedulePageRequest();
  }

  bool _onScrollNotification(ScrollNotification notification) {
    if (notification is ScrollUpdateNotification ||
        notification is ScrollEndNotification) {
      widget.onPositionChanged(
        notification.metrics,
        _previewDirectionFor(notification),
      );
      final controller = widget.scrollController;
      if (notification is ScrollUpdateNotification &&
          (notification.dragDetails != null ||
              controller is AmeScrollController &&
                  controller.isUserScrolling)) {
        _pageDemand.scroll(widget.state.query, notification.scrollDelta ?? 0);
      }
      _schedulePageRequest();
    }
    return false;
  }

  static LibraryPreviewMovementDirection _previewDirectionFor(
    ScrollNotification notification,
  ) {
    if (notification case ScrollUpdateNotification(:final scrollDelta?)) {
      if (scrollDelta > 0) {
        return LibraryPreviewMovementDirection.forward;
      }
      if (scrollDelta < 0) {
        return LibraryPreviewMovementDirection.backward;
      }
    }
    return LibraryPreviewMovementDirection.idle;
  }

  void _schedulePageRequest() {
    if (_pageCheck != null || !_pageDemand.hasDemand) return;
    _pageCheck = Timer(Duration.zero, () async {
      _pageCheck = null;
      if (!mounted ||
          !widget.scrollController.hasClients ||
          !TickerMode.getValuesNotifier(context).value.enabled) {
        return;
      }
      final metrics = widget.scrollController.position;
      if (!metrics.hasContentDimensions) return;
      // Read the resulting position, after Scrollable has handled the signal.
      final direction = _pageDemand.requestFor(
        widget.state,
        nearStart: _isNearLoadedStart(metrics),
        nearEnd: _isNearLoadedEnd(metrics),
      );
      switch (direction) {
        case LibraryPageDirection.previous:
          await widget.onLoadPrevious();
        case LibraryPageDirection.next:
          await widget.onLoadNext();
        case null:
          break;
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    return AmeScrollMotion(
      controller: widget.scrollController,
      onPointerSignal: _onPointerSignal,
      child: NotificationListener<ScrollMetricsNotification>(
        onNotification: (notification) {
          widget.onMetricsChanged(notification.metrics);
          _schedulePageRequest();
          return false;
        },
        child: NotificationListener<ScrollNotification>(
          onNotification: _onScrollNotification,
          child: widget.child,
        ),
      ),
    );
  }
}
