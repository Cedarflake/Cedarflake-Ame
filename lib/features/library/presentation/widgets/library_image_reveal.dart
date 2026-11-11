import "package:material_ui/material_ui.dart";

import "library_viewer_launch.dart";

/// Reveals decoded pixels without animating the card's geometry or compositing the whole wall.
class LibraryImageReveal extends StatefulWidget {
  const LibraryImageReveal({
    required this.image,
    required this.revealIdentity,
    this.fit = BoxFit.cover,
    this.filterQuality = FilterQuality.low,
    this.semanticLabel,
    this.frameBuilder,
    this.placeholder,
    this.errorBuilder,
    this.deferReveal = false,
    this.duration = const Duration(milliseconds: 120),
    super.key,
  });

  final ImageProvider<Object> image;
  final Object revealIdentity;
  final BoxFit fit;
  final FilterQuality filterQuality;
  final String? semanticLabel;
  final ImageFrameBuilder? frameBuilder;
  final Widget? placeholder;
  final ImageErrorWidgetBuilder? errorBuilder;
  final bool deferReveal;
  final Duration duration;

  @override
  State<LibraryImageReveal> createState() => _LibraryImageRevealState();
}

class _LibraryImageRevealState extends State<LibraryImageReveal>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;
  late final Animation<double> _opacity;
  ScrollPosition? _position;
  DateTime? _lastScrollAt;
  bool _hasFrame = false;
  bool _skipMotion = false;
  LibraryViewerThumbnailScope? _thumbnail;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: widget.duration,
      value: widget.deferReveal ? 0 : 1,
    );
    _opacity = _controller.drive(CurveTween(curve: Curves.easeOut));
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final thumbnail = LibraryViewerThumbnailScope.of(context);
    if (_thumbnail != thumbnail) {
      _thumbnail?.retire(this);
      _thumbnail = thumbnail;
    }
    final disableAnimations = MediaQuery.disableAnimationsOf(context);
    _skipMotion = disableAnimations || !TickerMode.valuesOf(context).enabled;
    final position = Scrollable.maybeOf(context)?.position;
    if (!identical(position, _position)) {
      _position?.removeListener(_handleScroll);
      _position = position;
      _position?.addListener(_handleScroll);
    }
    // Muting a retained surface freezes its visible blend during viewer entry/exit.
    // Only the accessibility preference should finish that blend immediately.
    if (disableAnimations) {
      _controller.value = 1;
    }
  }

  @override
  void didUpdateWidget(covariant LibraryImageReveal oldWidget) {
    super.didUpdateWidget(oldWidget);
    _controller.duration = widget.duration;
    if (oldWidget.revealIdentity != widget.revealIdentity) {
      _hasFrame = false;
      _controller.value = widget.deferReveal ? 0 : 1;
    }
    if (widget.deferReveal) {
      _controller.stop();
      if (!_hasFrame) _controller.value = 0;
    } else if (oldWidget.deferReveal && _hasFrame && _controller.value < 1) {
      _controller.forward();
    }
  }

  void _handleScroll() {
    _lastScrollAt = DateTime.now();
    if (_controller.isAnimating) {
      _controller.value = 1;
    }
  }

  Widget _buildFrame(
    BuildContext context,
    Widget child,
    int? frame,
    bool synchronous,
  ) {
    if (frame != null || synchronous) {
      _thumbnail?.publish(this, widget.image, widget.filterQuality);
    }
    if (!_hasFrame && !widget.deferReveal && (frame != null || synchronous)) {
      _hasFrame = true;
      final lastScrollAt = _lastScrollAt;
      final recentlyScrolled =
          lastScrollAt != null &&
          DateTime.now().difference(lastScrollAt) < widget.duration;
      if ((synchronous && _controller.value != 0) ||
          _skipMotion ||
          recentlyScrolled ||
          _position?.isScrollingNotifier.value == true) {
        _controller.value = 1;
      } else {
        _controller.forward(from: 0);
      }
    }
    final placeholder = widget.placeholder;
    final pixels = placeholder != null
        ? AnimatedBuilder(
            animation: _opacity,
            child: child,
            builder: (context, image) => Stack(
              fit: StackFit.expand,
              children: [
                if (!_hasFrame || _opacity.value < 1) placeholder,
                image!,
              ],
            ),
          )
        : child;
    return widget.frameBuilder?.call(context, pixels, frame, synchronous) ??
        pixels;
  }

  @override
  Widget build(BuildContext context) => Image(
    image: widget.image,
    fit: widget.fit,
    filterQuality: widget.filterQuality,
    semanticLabel: widget.semanticLabel,
    gaplessPlayback: true,
    opacity: _opacity,
    frameBuilder: _buildFrame,
    errorBuilder: widget.errorBuilder == null
        ? null
        : (context, error, stackTrace) => widget.deferReveal && !_hasFrame
              ? _buildFrame(context, const SizedBox.shrink(), null, false)
              : widget.errorBuilder!(context, error, stackTrace),
  );

  @override
  void dispose() {
    _thumbnail?.retire(this);
    _position?.removeListener(_handleScroll);
    _controller.dispose();
    super.dispose();
  }
}
