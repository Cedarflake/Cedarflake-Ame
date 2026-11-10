import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_motion.dart";
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
    with TickerProviderStateMixin {
  late final AnimationController _controller;
  late final AnimationController _opacity;
  Widget? _secondary;
  bool get _skipMotion =>
      MediaQuery.disableAnimationsOf(context) ||
      !TickerMode.valuesOf(context).enabled;

  @override
  void initState() {
    super.initState();
    _secondary = widget.secondary;
    _controller = AnimationController(
      vsync: this,
      value: _secondary == null ? 0 : 1,
    )..addStatusListener(_onStatusChanged);
    _opacity = AnimationController(
      vsync: this,
      value: _secondary == null ? 0 : 1,
    )..addStatusListener(_onStatusChanged);
  }

  void _onStatusChanged(AnimationStatus status) {
    if (status == AnimationStatus.dismissed &&
        widget.secondary == null &&
        _controller.isDismissed &&
        _opacity.isDismissed &&
        _secondary != null) {
      setState(() => _secondary = null);
    }
  }

  @override
  void didUpdateWidget(covariant LibraryGalleryHeaderActions oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.secondary != null) _secondary = widget.secondary;
    if (_skipMotion) {
      _finish();
    } else if ((oldWidget.secondary == null) != (widget.secondary == null)) {
      final forward = widget.secondary != null;
      AmeMotion.spatial.animate(_controller, forward: forward);
      AmeMotion.effects.animate(_opacity, forward: forward);
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_skipMotion) _finish();
  }

  void _finish() {
    final target = widget.secondary == null ? 0.0 : 1.0;
    _controller.value = target;
    _opacity.value = target;
  }

  @override
  void dispose() {
    _controller.dispose();
    _opacity.dispose();
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
          child: FadeTransition(opacity: _opacity, child: controls),
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
                  padding: const EdgeInsets.only(
                    top: LibraryGalleryHeaderRows.rowSpacing,
                  ),
                  child: controls,
                ),
              ),
          ],
        ),
      ),
    );
  }
}
