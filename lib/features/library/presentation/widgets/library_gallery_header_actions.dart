import "package:flutter/material.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "library_gallery_header_rows.dart";

class LibraryGalleryHeaderActions extends StatefulWidget {
  const LibraryGalleryHeaderActions({
    required this.primary,
    this.secondary,
    this.title,
    this.summary,
    this.minimumWidth = 0,
    super.key,
  }) : assert((title == null) == (summary == null));

  final Widget primary;
  final Widget? secondary;
  final Widget? title;
  final Widget? summary;
  final double minimumWidth;

  @override
  State<LibraryGalleryHeaderActions> createState() =>
      _LibraryGalleryHeaderActionsState();
}

class _LibraryGalleryHeaderActionsState
    extends State<LibraryGalleryHeaderActions>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  Widget? _secondary;

  @override
  void initState() {
    super.initState();
    _secondary = widget.secondary;
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 260),
      value: _secondary == null ? 0 : 1,
    )..addStatusListener(_onStatusChanged);
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
    } else if ((oldWidget.secondary == null) != (widget.secondary == null)) {
      final duration =
          _controller.duration! * (target - _controller.value).abs();
      if (widget.secondary != null) {
        _controller.animateTo(
          target,
          duration: duration,
          curve: Curves.easeOutCubic,
        );
      } else {
        _controller.animateBack(
          target,
          duration: duration,
          curve: Curves.easeOutCubic,
        );
      }
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (MediaQuery.disableAnimationsOf(context)) {
      _controller.value = widget.secondary == null ? 0 : 1;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Widget _scrollable(Widget child) => AmeScrollMotion(
    scrollDirection: Axis.horizontal,
    child: SingleChildScrollView(
      scrollDirection: Axis.horizontal,
      reverse: true,
      child: child,
    ),
  );

  Widget? _secondaryControls() {
    final controls = _secondary;
    if (controls == null) return null;
    return IgnorePointer(
      ignoring: widget.secondary == null,
      child: ExcludeFocus(
        excluding: widget.secondary == null,
        child: ExcludeSemantics(
          excluding: widget.secondary == null,
          child: FadeTransition(opacity: _controller, child: controls),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final controls = _secondaryControls();
    if (widget.title case final title?) {
      return AnimatedBuilder(
        animation: _controller,
        builder: (context, _) => LibraryGalleryHeaderRows(
          title: title,
          summary: widget.summary!,
          primaryActions: _scrollable(widget.primary),
          secondaryActions: controls == null
              ? const SizedBox.shrink()
              : _scrollable(controls),
          progress: _controller.value,
        ),
      );
    }
    return _scrollable(
      ConstrainedBox(
        constraints: BoxConstraints(minWidth: widget.minimumWidth),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.end,
          children: [
            widget.primary,
            if (controls != null)
              SizeTransition(
                sizeFactor: _controller,
                alignment: Alignment.topRight,
                child: Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: controls,
                ),
              ),
          ],
        ),
      ),
    );
  }
}
