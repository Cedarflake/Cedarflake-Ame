import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_motion.dart";
import "../../application/library_workbench_controller.dart";
import "library_main_surface.dart";
import "library_sidebar_content.dart";
import "library_workbench_metrics.dart";

/// Carries the current content boundary across retained workspace changes.
class LibraryWorkspaceWidthTransition extends StatefulWidget {
  const LibraryWorkspaceWidthTransition({
    required this.destination,
    required this.contentInset,
    required this.child,
    super.key,
  });

  final LibraryWorkspace destination;
  final double contentInset;
  final Widget child;

  @override
  State<LibraryWorkspaceWidthTransition> createState() =>
      _LibraryWorkspaceWidthTransitionState();
}

class _LibraryWorkspaceWidthTransitionState
    extends State<LibraryWorkspaceWidthTransition>
    with SingleTickerProviderStateMixin {
  late final _controller = AnimationController(vsync: this, value: 1);
  Animation<double> _offset = const AlwaysStoppedAnimation(0);

  bool get _skipMotion =>
      MediaQuery.disableAnimationsOf(context) ||
      !TickerMode.valuesOf(context).enabled;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_skipMotion) _controller.value = 1;
  }

  @override
  void didUpdateWidget(covariant LibraryWorkspaceWidthTransition oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_skipMotion || widget.destination == oldWidget.destination) {
      if (_skipMotion || widget.contentInset != oldWidget.contentInset) {
        _controller.value = 1;
      }
      return;
    }
    final displayedInset = oldWidget.contentInset + _offset.value;
    if ((displayedInset - widget.contentInset).abs() < 0.01) {
      _controller.value = 1;
      _offset = const AlwaysStoppedAnimation(0);
      return;
    }
    _offset = Tween<double>(
      begin: displayedInset - widget.contentInset,
      end: 0,
    ).animate(_controller);
    _controller.value = 0;
    AmeMotion.spatial.animate(_controller, forward: true);
  }

  @override
  Widget build(BuildContext context) => LibraryWorkspaceWidthMotion(
    destination: widget.destination,
    offset: _offset,
    child: widget.child,
  );

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }
}

class LibraryWorkspaceWidthMotion extends InheritedWidget {
  const LibraryWorkspaceWidthMotion({
    required this.destination,
    required this.offset,
    required super.child,
    super.key,
  });

  final Animation<double> offset;
  final LibraryWorkspace destination;

  static Animation<double>? of(
    BuildContext context, {
    LibraryWorkspace? destination,
  }) {
    final motion = context
        .dependOnInheritedWidgetOfExactType<LibraryWorkspaceWidthMotion>();
    if (destination != null && motion?.destination != destination) return null;
    return motion?.offset;
  }

  @override
  bool updateShouldNotify(LibraryWorkspaceWidthMotion oldWidget) =>
      offset != oldWidget.offset || destination != oldWidget.destination;
}

/// Keeps settings at its destination width while its visible pane expands.
class LibraryWorkspaceContentPane extends StatelessWidget {
  const LibraryWorkspaceContentPane({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final offset = LibraryWorkspaceWidthMotion.of(
      context,
      destination: LibraryWorkspace.settings,
    );
    const inset = LibraryWorkbenchMetrics.contentInset;
    return LayoutBuilder(
      builder: (context, constraints) => AnimatedBuilder(
        animation: offset ?? const AlwaysStoppedAnimation(0.0),
        child: LibrarySidebarMotion(
          progress: const AlwaysStoppedAnimation(1),
          sidebarWidth: inset,
          expanded: true,
          workspaceOffset: offset,
          child: LibrarySidebarContent(
            alignment: Alignment.topCenter,
            child: RepaintBoundary(child: child),
          ),
        ),
        builder: (context, child) => Stack(
          fit: StackFit.expand,
          children: [
            Positioned(
              left: inset + (offset?.value ?? 0),
              right: 0,
              top: 0,
              bottom: 0,
              child: LibraryMainSurface(
                child: OverflowBox(
                  alignment: Alignment.topRight,
                  minWidth: constraints.maxWidth - inset,
                  maxWidth: constraints.maxWidth - inset,
                  child: child,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
