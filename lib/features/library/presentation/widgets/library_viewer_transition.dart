import "package:flutter/material.dart";

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
  late final Animation<double> _opacity;
  Widget? _retainedViewer;
  bool _skipMotion = false;

  @override
  void initState() {
    super.initState();
    _retainedViewer = widget.viewer;
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 180),
      reverseDuration: const Duration(milliseconds: 100),
      value: widget.viewer == null ? 0 : 1,
    )..addStatusListener(_handleStatus);
    _opacity = _controller.drive(CurveTween(curve: Curves.easeOut));
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _skipMotion =
        MediaQuery.disableAnimationsOf(context) || !TickerMode.of(context);
    if (_skipMotion) _finishTransition();
  }

  @override
  void didUpdateWidget(covariant LibraryViewerTransition oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.viewer != null) _retainedViewer = widget.viewer;
    if (_skipMotion) {
      _finishTransition();
    } else if (oldWidget.viewer == null && widget.viewer != null) {
      _controller.forward();
    } else if (oldWidget.viewer != null && widget.viewer == null) {
      _controller.reverse();
    }
  }

  void _finishTransition() {
    _controller.value = widget.viewer == null ? 0 : 1;
    if (widget.viewer == null) _retainedViewer = null;
  }

  void _handleStatus(AnimationStatus status) {
    if (status == AnimationStatus.dismissed ||
        status == AnimationStatus.completed) {
      setState(() {
        if (status == AnimationStatus.dismissed) _retainedViewer = null;
      });
    }
  }

  void _handleInput(PointerEvent event) {
    if (widget.viewer != null && _controller.isAnimating) _controller.value = 1;
  }

  @override
  Widget build(BuildContext context) {
    final isOpen = widget.viewer != null;
    return Stack(
      fit: StackFit.expand,
      children: [
        Offstage(
          offstage: isOpen && _controller.isCompleted,
          child: TickerMode(
            enabled: !isOpen,
            child: ExcludeFocus(
              excluding: isOpen,
              child: ExcludeSemantics(
                excluding: isOpen,
                child: IgnorePointer(ignoring: isOpen, child: widget.gallery),
              ),
            ),
          ),
        ),
        if (_retainedViewer case final viewer?)
          FadeTransition(
            opacity: _opacity,
            child: IgnorePointer(
              ignoring: !isOpen,
              child: ExcludeFocus(
                excluding: !isOpen,
                child: ExcludeSemantics(
                  excluding: !isOpen,
                  child: LibraryViewerActivity(
                    isActive: isOpen,
                    child: TickerMode(
                      enabled: isOpen,
                      child: Listener(
                        onPointerDown: _handleInput,
                        onPointerSignal: _handleInput,
                        child: ColoredBox(
                          color: Theme.of(
                            context,
                          ).colorScheme.surfaceContainerLow,
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
