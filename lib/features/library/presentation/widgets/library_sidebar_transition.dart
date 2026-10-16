import "package:flutter/material.dart";

import "library_main_surface.dart";
import "library_navigation_resize_handle.dart";

/// Animates the left boundary of a right-anchored, destination-width gallery.
class LibrarySidebarTransition extends StatefulWidget {
  const LibrarySidebarTransition({
    required this.expanded,
    required this.sidebarWidth,
    required this.sidebar,
    required this.gallery,
    required this.resizeHandle,
    super.key,
  });

  final bool expanded;
  final double sidebarWidth;
  final Widget sidebar;
  final Widget gallery;
  final Widget resizeHandle;

  @override
  State<LibrarySidebarTransition> createState() =>
      _LibrarySidebarTransitionState();
}

class _LibrarySidebarTransitionState extends State<LibrarySidebarTransition>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _progress;
  bool _skipMotion = false;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 180),
      value: widget.expanded ? 1 : 0,
    );
    _progress = _controller.drive(CurveTween(curve: Curves.easeInOutCubic));
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _skipMotion =
        MediaQuery.disableAnimationsOf(context) ||
        !TickerMode.valuesOf(context).enabled;
    if (_skipMotion) _finish();
  }

  @override
  void didUpdateWidget(covariant LibrarySidebarTransition oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_skipMotion) {
      _finish();
    } else if (widget.expanded != oldWidget.expanded) {
      if (widget.expanded) {
        _controller.forward();
      } else {
        _controller.reverse();
      }
    }
  }

  void _finish() => _controller.value = widget.expanded ? 1 : 0;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final galleryWidth =
          constraints.maxWidth - (widget.expanded ? widget.sidebarWidth : 0);
      return AnimatedBuilder(
        animation: _progress,
        child: RepaintBoundary(child: widget.gallery),
        builder: (context, gallery) {
          final extent = widget.sidebarWidth * _progress.value;
          return Stack(
            fit: StackFit.expand,
            children: [
              Positioned(
                left: 0,
                top: 0,
                bottom: 0,
                width: extent,
                child: ClipRect(
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
                child: ClipRRect(
                  key: const Key("library-gallery-transition"),
                  borderRadius: LibraryMainSurface.borderRadius,
                  child: ColoredBox(
                    color: Theme.of(context).colorScheme.surfaceContainerLowest,
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
    },
  );
}
