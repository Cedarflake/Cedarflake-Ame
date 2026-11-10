import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_motion.dart";
import "library_main_surface.dart";
import "library_navigation_resize_handle.dart";
import "library_sidebar_content.dart";

/// Animates the left boundary of a right-anchored, destination-width gallery.
class LibrarySidebarTransition extends StatefulWidget {
  const LibrarySidebarTransition({
    required this.expanded,
    required this.sidebarWidth,
    required this.sidebar,
    required this.gallery,
    required this.resizeHandle,
    this.frameBuilder,
    super.key,
  });

  final bool expanded;
  final double sidebarWidth;
  final Widget sidebar;
  final Widget gallery;
  final Widget resizeHandle;
  final Widget Function(BuildContext context, double extent, Widget panels)?
  frameBuilder;

  @override
  State<LibrarySidebarTransition> createState() =>
      _LibrarySidebarTransitionState();
}

class _LibrarySidebarTransitionState extends State<LibrarySidebarTransition>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _progress;
  bool get _skipMotion =>
      MediaQuery.disableAnimationsOf(context) ||
      !TickerMode.valuesOf(context).enabled;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      value: widget.expanded ? 1 : 0,
    );
    _progress = _controller.view;
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_skipMotion) _finish();
  }

  @override
  void didUpdateWidget(covariant LibrarySidebarTransition oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_skipMotion) {
      _finish();
    } else if (widget.expanded != oldWidget.expanded) {
      AmeMotion.spatial.animate(_controller, forward: widget.expanded);
    }
  }

  void _finish() => _controller.value = widget.expanded ? 1 : 0;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: _progress,
    child: LibrarySidebarMotion(
      progress: _progress,
      sidebarWidth: widget.sidebarWidth,
      expanded: widget.expanded,
      child: RepaintBoundary(child: widget.gallery),
    ),
    builder: (context, gallery) {
      final extent = widget.sidebarWidth * _progress.value;
      final panels = LayoutBuilder(
        builder: (context, constraints) {
          final galleryWidth =
              constraints.maxWidth -
              (widget.expanded ? widget.sidebarWidth : 0);
          return Stack(
            fit: StackFit.expand,
            children: [
              Positioned(
                left: 0,
                top: 0,
                bottom: 0,
                width: extent,
                child: ClipRRect(
                  borderRadius: LibraryMainSurface.borderRadius,
                  child: OverflowBox(
                    alignment: Alignment.topLeft,
                    minWidth: widget.sidebarWidth,
                    maxWidth: widget.sidebarWidth,
                    child: Visibility(
                      visible: extent > 0,
                      maintainState: true,
                      child: ExcludeFocus(
                        excluding: !widget.expanded,
                        child: IgnorePointer(
                          ignoring: !widget.expanded,
                          child: widget.sidebar,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
              Positioned(
                left: extent,
                right: 0,
                top: 0,
                bottom: 0,
                child: LibraryMainSurface(
                  key: const Key("library-gallery-transition"),
                  // Keep right-edge controls stationary as the left clip moves.
                  // Reflowing the gallery at every intermediate width is costly.
                  child: OverflowBox(
                    alignment: Alignment.topRight,
                    minWidth: galleryWidth,
                    maxWidth: galleryWidth,
                    child: gallery,
                  ),
                ),
              ),
              if (widget.expanded && _controller.isCompleted)
                Positioned(
                  left:
                      extent - LibraryNavigationResizeHandle.hitTargetWidth / 2,
                  top: 0,
                  bottom: 0,
                  child: widget.resizeHandle,
                ),
            ],
          );
        },
      );
      return widget.frameBuilder?.call(context, extent, panels) ?? panels;
    },
  );
}
