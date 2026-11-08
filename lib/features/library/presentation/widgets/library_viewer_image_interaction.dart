import "package:flutter/gestures.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter/services.dart";

import "../../../settings/domain/ame_preferences.dart";
import "../coordination/library_viewer_zoom.dart";

Map<ShortcutActivator, VoidCallback> libraryViewerZoomShortcuts(
  LibraryViewerZoom zoom,
) => {
  const SingleActivator(LogicalKeyboardKey.add): zoom.zoomIn,
  const SingleActivator(LogicalKeyboardKey.equal, shift: true): zoom.zoomIn,
  const SingleActivator(LogicalKeyboardKey.minus): zoom.zoomOut,
  const SingleActivator(LogicalKeyboardKey.numpadAdd): zoom.zoomIn,
  const SingleActivator(LogicalKeyboardKey.numpadSubtract): zoom.zoomOut,
  const SingleActivator(LogicalKeyboardKey.equal, control: true): zoom.zoomIn,
  const SingleActivator(LogicalKeyboardKey.equal, control: true, shift: true):
      zoom.zoomIn,
  const SingleActivator(LogicalKeyboardKey.minus, control: true): zoom.zoomOut,
  const SingleActivator(LogicalKeyboardKey.digit0): zoom.fit,
  const SingleActivator(LogicalKeyboardKey.digit0, control: true): zoom.fit,
  const SingleActivator(LogicalKeyboardKey.digit1): zoom.actualSize,
  const SingleActivator(LogicalKeyboardKey.digit1, control: true):
      zoom.actualSize,
};

class LibraryViewerImageInteraction extends StatefulWidget {
  const LibraryViewerImageInteraction({
    required this.zoom,
    required this.child,
    this.wheelBehavior = ImageViewerWheelBehavior.zoom,
    this.onPrevious,
    this.onNext,
    this.interactiveKey = const Key("library-image-interactive-viewer"),
    super.key,
  });

  final LibraryViewerZoom zoom;
  final Widget child;
  final ImageViewerWheelBehavior wheelBehavior;
  final VoidCallback? onPrevious;
  final VoidCallback? onNext;
  final Key interactiveKey;

  @override
  State<LibraryViewerImageInteraction> createState() =>
      _LibraryViewerImageInteractionState();
}

class _LibraryViewerImageInteractionState
    extends State<LibraryViewerImageInteraction> {
  DateTime? _lastWheelNavigationAt;

  void _handlePointerSignal(PointerSignalEvent event) {
    if (event is! PointerScrollEvent || event.scrollDelta.dy == 0) return;
    if (widget.wheelBehavior == ImageViewerWheelBehavior.zoom) {
      widget.zoom.animatePointerZoom();
      return;
    }
    final now = DateTime.now();
    final lastNavigation = _lastWheelNavigationAt;
    if (lastNavigation != null &&
        now.difference(lastNavigation) < const Duration(milliseconds: 240)) {
      return;
    }
    _lastWheelNavigationAt = now;
    if (event.scrollDelta.dy < 0) {
      widget.onPrevious?.call();
    } else {
      widget.onNext?.call();
    }
  }

  @override
  Widget build(BuildContext context) => Listener(
    onPointerSignal: _handlePointerSignal,
    child: GestureDetector(
      behavior: HitTestBehavior.opaque,
      onDoubleTap: widget.zoom.toggleMode,
      child: InteractiveViewer(
        key: widget.interactiveKey,
        transformationController: widget.zoom.transform,
        alignment: Alignment.topLeft,
        boundaryMargin: const EdgeInsets.all(double.infinity),
        minScale: widget.zoom.minimumScale,
        maxScale: widget.zoom.maximumScale,
        scaleFactor: 180,
        scaleEnabled: widget.wheelBehavior == ImageViewerWheelBehavior.zoom,
        trackpadScrollCausesScale: true,
        onInteractionStart: widget.zoom.interactionStarted,
        child: SizedBox(
          width: widget.zoom.viewport.width,
          height: widget.zoom.viewport.height,
          child: widget.child,
        ),
      ),
    ),
  );
}
