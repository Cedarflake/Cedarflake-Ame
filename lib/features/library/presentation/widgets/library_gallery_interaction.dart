import "dart:async";

import "package:flutter/gestures.dart";
import "package:flutter/widgets.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../domain/library_state.dart";
import "library_gallery_viewport_feedback.dart";
import "library_virtual_gallery_geometry.dart";

class LibraryGalleryInteraction extends StatelessWidget {
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

  bool get _canLoadPrevious =>
      state.hasPreviousAssets &&
      !state.isLoadingPage &&
      !state.isLoadingPreviousPage &&
      state.previousPageErrorMessage == null;

  bool get _canLoadNext =>
      state.hasMoreAssets &&
      !state.isLoadingPage &&
      !state.isLoadingPreviousPage &&
      state.pageErrorMessage == null;

  bool _isNearLoadedStart(ScrollMetrics metrics) {
    return metrics.pixels >=
            geometry.leadingExtent - metrics.viewportDimension &&
        metrics.pixels <= geometry.leadingExtent + 900;
  }

  bool _isNearLoadedEnd(ScrollMetrics metrics) {
    final visibleBottom = metrics.pixels + metrics.viewportDimension;
    return visibleBottom >= geometry.loadedEndExtent - 900 &&
        metrics.pixels <= geometry.loadedEndExtent;
  }

  void _onPointerSignal(BuildContext context, PointerSignalEvent event) {
    if (event is! PointerScrollEvent || !scrollController.hasClients) {
      return;
    }
    final metrics = scrollController.position;
    if (event.scrollDelta.dy < 0 &&
        _isNearLoadedStart(metrics) &&
        _canLoadPrevious) {
      _deferPageRequest(context, onLoadPrevious);
    } else if (event.scrollDelta.dy > 0 &&
        _isNearLoadedEnd(metrics) &&
        _canLoadNext) {
      _deferPageRequest(context, onLoadNext);
    }
  }

  bool _onScrollNotification(
    BuildContext context,
    ScrollNotification notification,
  ) {
    if (notification is ScrollUpdateNotification ||
        notification is ScrollEndNotification) {
      onPositionChanged(
        notification.metrics,
        _previewDirectionFor(notification),
      );
      final isDirectDragUpdate =
          notification is ScrollUpdateNotification &&
          notification.dragDetails != null;
      if (isDirectDragUpdate &&
          _isNearLoadedStart(notification.metrics) &&
          _canLoadPrevious) {
        _deferPageRequest(context, onLoadPrevious);
      }
      if (isDirectDragUpdate &&
          _isNearLoadedEnd(notification.metrics) &&
          _canLoadNext) {
        _deferPageRequest(context, onLoadNext);
      }
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

  @override
  Widget build(BuildContext context) {
    return AmeScrollMotion(
      controller: scrollController,
      onPointerSignal: (event) => _onPointerSignal(context, event),
      child: NotificationListener<ScrollMetricsNotification>(
        onNotification: (notification) {
          onMetricsChanged(notification.metrics);
          return false;
        },
        child: NotificationListener<ScrollNotification>(
          onNotification: (notification) =>
              _onScrollNotification(context, notification),
          child: child,
        ),
      ),
    );
  }
}
