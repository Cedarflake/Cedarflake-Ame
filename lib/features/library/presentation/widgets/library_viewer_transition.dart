import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_motion.dart";
import "library_viewer_fade.dart";
import "library_viewer_launch.dart";

/// Retains at most one closing surface; its input and source reads retire immediately.
class LibraryViewerTransition extends StatefulWidget {
  const LibraryViewerTransition({
    required this.gallery,
    required this.viewer,
    super.key,
  });

  final Widget gallery;
  final Widget? viewer;

  @override
  State<LibraryViewerTransition> createState() =>
      _LibraryViewerTransitionState();
}

class _LibraryViewerTransitionState extends State<LibraryViewerTransition>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  Widget? _retainedViewer;
  LibraryViewerOrigin? _origin;
  final _origins = LibraryViewerOrigins();
  bool get _skipMotion =>
      MediaQuery.disableAnimationsOf(context) ||
      !TickerMode.valuesOf(context).enabled;

  @override
  void initState() {
    super.initState();
    _retainedViewer = widget.viewer;
    _controller = AnimationController(
      vsync: this,
      value: widget.viewer == null ? 0 : 1,
    )..addStatusListener(_handleStatus);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_skipMotion) _finishTransition();
  }

  @override
  void didUpdateWidget(covariant LibraryViewerTransition oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.viewer != null) _retainedViewer = widget.viewer;
    if (_skipMotion) {
      _finishTransition();
    } else if (oldWidget.viewer == null && widget.viewer != null) {
      _origin = _origin?.resolve();
      (_origin == null ? LibraryViewerFade.enterMotion : AmeMotion.spatial)
          .animate(_controller, forward: true);
    } else if (oldWidget.viewer != null && widget.viewer == null) {
      (_origin == null ? LibraryViewerFade.exitMotion : AmeMotion.fastSpatial)
          .animate(_controller, forward: false);
    }
  }

  void _finishTransition() {
    _controller.value = widget.viewer == null ? 0 : 1;
    if (widget.viewer == null) {
      _retainedViewer = null;
      _origin = null;
    }
  }

  void _handleStatus(AnimationStatus status) {
    if (status == AnimationStatus.dismissed ||
        status == AnimationStatus.completed) {
      setState(() {
        if (status == AnimationStatus.dismissed) {
          _retainedViewer = null;
          _origin = null;
        }
      });
    }
  }

  void _handleInput(PointerEvent event) {
    if (widget.viewer != null && _controller.isAnimating) _controller.value = 1;
  }

  @override
  Widget build(BuildContext context) {
    final isOpen = widget.viewer != null;
    return LibraryViewerOriginScope(
      origins: _origins,
      prepare: (origin) => _origin = origin,
      take: () {
        final origin = _origin;
        _origin = null;
        return origin?.resolve();
      },
      child: Stack(
        fit: StackFit.expand,
        children: [
          Semantics(
            container: true,
            explicitChildNodes: true,
            hidden: isOpen,
            child: Opacity(
              opacity: isOpen && _controller.isCompleted ? 0 : 1,
              // Hidden gallery portals keep their traversal anchors for the return frame.
              alwaysIncludeSemantics: true,
              child: TickerMode(
                enabled: !isOpen,
                child: ExcludeFocus(
                  excluding: isOpen,
                  child: IgnorePointer(ignoring: isOpen, child: widget.gallery),
                ),
              ),
            ),
          ),
          if (_retainedViewer case final viewer?)
            Semantics(
              container: true,
              explicitChildNodes: true,
              hidden: !isOpen,
              child: LibraryViewerFade(
                animation: _origin == null
                    ? _controller
                    : const AlwaysStoppedAnimation(1),
                child: IgnorePointer(
                  ignoring: !isOpen,
                  child: ExcludeFocus(
                    excluding: !isOpen,
                    child: LibraryViewerActivity(
                      isActive: isOpen,
                      child: TickerMode(
                        enabled: isOpen,
                        child: Listener(
                          onPointerDown: _handleInput,
                          onPointerSignal: _handleInput,
                          child: LibraryViewerFlightScope(
                            origin: _origin,
                            animation: _controller,
                            child: viewer,
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }
}

class LibraryViewerActivity extends InheritedWidget {
  const LibraryViewerActivity({
    required this.isActive,
    required super.child,
    super.key,
  });

  final bool isActive;

  static bool isActiveOf(BuildContext context) =>
      context
          .dependOnInheritedWidgetOfExactType<LibraryViewerActivity>()
          ?.isActive ??
      true;

  @override
  bool updateShouldNotify(LibraryViewerActivity oldWidget) =>
      isActive != oldWidget.isActive;
}
