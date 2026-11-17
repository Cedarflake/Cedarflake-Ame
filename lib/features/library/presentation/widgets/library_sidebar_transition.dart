import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_motion.dart";
import "../../application/library_workbench_controller.dart";
import "library_main_surface.dart";
import "library_navigation_resize_handle.dart";
import "library_sidebar_content.dart";
import "library_workbench_metrics.dart";
import "library_workspace_width_transition.dart";

/// Animates the left boundary of a right-anchored, destination-width gallery.
class LibrarySidebarTransition extends StatefulWidget {
  const LibrarySidebarTransition({
    required this.expanded,
    required this.sidebarWidth,
    required this.sidebar,
    required this.gallery,
    this.resizeHandle,
    this.frameBuilder,
    this.workspace,
    super.key,
  });

  final bool expanded;
  final double sidebarWidth;
  final Widget sidebar;
  final Widget gallery;
  final Widget? resizeHandle;
  final LibraryWorkspace? workspace;
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
  LibraryWorkspace? _activeWorkspace;
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
    final workspace = LibraryWorkspaceWidthMotion.of(context)?.destination;
    if (_skipMotion ||
        (_activeWorkspace != null && workspace != _activeWorkspace)) {
      // Workspace motion takes over the visible boundary from sidebar motion.
      _finish();
    }
    _activeWorkspace = workspace;
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
  Widget build(BuildContext context) {
    final workspaceMotion = LibraryWorkspaceWidthMotion.of(
      context,
      destination: widget.workspace,
    );
    final workspaceOffset = workspaceMotion?.offset;
    return AnimatedBuilder(
      animation: Listenable.merge([_progress, ?workspaceOffset]),
      child: LibrarySidebarMotion(
        progress: _progress,
        sidebarWidth: widget.sidebarWidth,
        expanded: widget.expanded,
        workspaceOffset: workspaceOffset,
        child: RepaintBoundary(child: widget.gallery),
      ),
      builder: (context, gallery) {
        final extent =
            LibraryWorkbenchMetrics.contentInset +
            (widget.sidebarWidth - LibraryWorkbenchMetrics.contentInset) *
                _progress.value;
        final displayedInset = extent + (workspaceOffset?.value ?? 0);
        workspaceMotion?.recordInset(displayedInset);
        final panels = LayoutBuilder(
          builder: (context, constraints) {
            final galleryWidth =
                constraints.maxWidth -
                (widget.expanded
                    ? widget.sidebarWidth
                    : LibraryWorkbenchMetrics.contentInset);
            return Stack(
              fit: StackFit.expand,
              children: [
                Positioned(
                  left: 0,
                  top: 0,
                  bottom: 0,
                  width: widget.sidebarWidth * _progress.value,
                  child: ClipRect(
                    child: OverflowBox(
                      alignment: Alignment.topLeft,
                      minWidth: widget.sidebarWidth,
                      maxWidth: widget.sidebarWidth,
                      child: Material(
                        type: MaterialType.transparency,
                        child: Visibility(
                          visible: _progress.value > 0,
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
                ),
                Positioned(
                  left: displayedInset,
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
                if (widget.expanded &&
                    _controller.isCompleted &&
                    widget.resizeHandle != null)
                  Positioned(
                    left:
                        extent -
                        LibraryNavigationResizeHandle.hitTargetWidth / 2,
                    top: 0,
                    bottom: 0,
                    child: widget.resizeHandle!,
                  ),
              ],
            );
          },
        );
        return widget.frameBuilder?.call(context, extent, panels) ?? panels;
      },
    );
  }
}
