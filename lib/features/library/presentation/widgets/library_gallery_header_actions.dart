import "package:flutter/material.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";

class LibraryGalleryHeaderActions extends StatefulWidget {
  const LibraryGalleryHeaderActions({
    required this.primary,
    this.secondary,
    this.minimumWidth = 0,
    super.key,
  });

  final Widget primary;
  final Widget? secondary;
  final double minimumWidth;

  @override
  State<LibraryGalleryHeaderActions> createState() =>
      _LibraryGalleryHeaderActionsState();
}

class _LibraryGalleryHeaderActionsState
    extends State<LibraryGalleryHeaderActions>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _progress;
  Widget? _secondary;

  @override
  void initState() {
    super.initState();
    _secondary = widget.secondary;
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 180),
      value: _secondary == null ? 0 : 1,
    )..addStatusListener(_onStatusChanged);
    _progress = _controller.drive(CurveTween(curve: Curves.easeInOutCubic));
  }

  void _onStatusChanged(AnimationStatus status) {
    if (status == AnimationStatus.dismissed && widget.secondary == null) {
      setState(() => _secondary = null);
    }
  }

  @override
  void didUpdateWidget(covariant LibraryGalleryHeaderActions oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.secondary != null) _secondary = widget.secondary;
    final target = widget.secondary == null ? 0.0 : 1.0;
    if (MediaQuery.disableAnimationsOf(context)) {
      _controller.value = target;
    } else if (widget.secondary != null) {
      _controller.forward();
    } else {
      _controller.reverse();
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => AmeScrollMotion(
    scrollDirection: Axis.horizontal,
    child: SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      reverse: true,
      child: ConstrainedBox(
        constraints: BoxConstraints(minWidth: widget.minimumWidth),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            widget.primary,
            if (_secondary case final controls?)
              IgnorePointer(
                ignoring: widget.secondary == null,
                child: ExcludeFocus(
                  excluding: widget.secondary == null,
                  child: ExcludeSemantics(
                    excluding: widget.secondary == null,
                    child: SizeTransition(
                      sizeFactor: _progress,
                      alignment: Alignment.topRight,
                      child: FadeTransition(
                        opacity: _progress,
                        child: Padding(
                          padding: const EdgeInsets.only(top: 8),
                          child: controls,
                        ),
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    ),
  );
}
