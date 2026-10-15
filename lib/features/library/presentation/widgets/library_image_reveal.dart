import "package:flutter/material.dart";

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

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: widget.duration,
      value: 1,
    );
    _opacity = _controller.drive(CurveTween(curve: Curves.easeOut));
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _skipMotion =
        MediaQuery.disableAnimationsOf(context) || !TickerMode.of(context);
    final position = Scrollable.maybeOf(context)?.position;
    if (!identical(position, _position)) {
      _position?.removeListener(_handleScroll);
      _position = position;
      _position?.addListener(_handleScroll);
    }
    if (_skipMotion) {
      _controller.value = 1;
    }
  }

  @override
  void didUpdateWidget(covariant LibraryImageReveal oldWidget) {
    super.didUpdateWidget(oldWidget);
    _controller.duration = widget.duration;
    if (oldWidget.revealIdentity != widget.revealIdentity) {
      _hasFrame = false;
      _controller.value = 1;
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
    if (!_hasFrame && (frame != null || synchronous)) {
      _hasFrame = true;
      final lastScrollAt = _lastScrollAt;
      final recentlyScrolled =
          lastScrollAt != null &&
          DateTime.now().difference(lastScrollAt) < widget.duration;
      if (synchronous ||
          _skipMotion ||
          recentlyScrolled ||
          _position?.isScrollingNotifier.value == true) {
        _controller.value = 1;
      } else {
        _controller.forward(from: 0);
      }
    }
    final placeholder = widget.placeholder;
    final pixels = placeholder != null && _hasFrame && !synchronous
        ? AnimatedBuilder(
            animation: _opacity,
            child: child,
            builder: (context, image) => Stack(
              fit: StackFit.expand,
              children: [if (_opacity.value < 1) placeholder, image!],
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
    errorBuilder: widget.errorBuilder,
  );

  @override
  void dispose() {
    _position?.removeListener(_handleScroll);
    _controller.dispose();
    super.dispose();
  }
}
