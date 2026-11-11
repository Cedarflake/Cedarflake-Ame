import "dart:async";

import "package:material_ui/material_ui.dart";

import "ame_motion.dart";
import "ame_loading_indicator.dart";

const ameProgressDelay = Duration(milliseconds: 250);

/// Delays visual feedback only; operation admission and completion stay immediate.
class AmeProgressVisibility extends StatefulWidget {
  const AmeProgressVisibility({
    required this.child,
    this.active = true,
    this.delay = ameProgressDelay,
    super.key,
  });

  final Widget child;
  final bool active;
  final Duration delay;

  @override
  State<AmeProgressVisibility> createState() => _AmeProgressVisibilityState();
}

class _AmeProgressVisibilityState extends State<AmeProgressVisibility>
    with SingleTickerProviderStateMixin {
  late final AnimationController _opacity;
  Timer? _delay;
  bool _shown = false;

  @override
  void initState() {
    super.initState();
    _opacity = AnimationController(vsync: this);
    if (widget.active) _schedule();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (MediaQuery.disableAnimationsOf(context)) {
      _opacity.value = _shown ? 1 : 0;
    }
  }

  @override
  void didUpdateWidget(covariant AmeProgressVisibility oldWidget) {
    super.didUpdateWidget(oldWidget);
    // A phase or delay change must not restart an already active interval.
    if (widget.active == oldWidget.active) return;
    _delay?.cancel();
    _delay = null;
    if (!widget.active) {
      _shown = false;
      _animate(false);
    } else if (_opacity.value > 0) {
      _shown = true;
      _animate(true);
    } else {
      _schedule();
    }
  }

  void _schedule() {
    if (widget.delay == Duration.zero) {
      _shown = true;
      _opacity.value = 1;
      return;
    }
    _delay = Timer(widget.delay, () {
      _delay = null;
      if (!mounted || !widget.active) return;
      setState(() => _shown = true);
      _animate(true);
    });
  }

  void _animate(bool visible) {
    if (MediaQuery.disableAnimationsOf(context)) {
      _opacity.value = visible ? 1 : 0;
    } else {
      AmeMotion.fastEffects.animate(_opacity, forward: visible);
    }
  }

  @override
  Widget build(BuildContext context) => IgnorePointer(
    child: ExcludeSemantics(
      excluding: !_shown || !widget.active,
      child: FadeTransition(
        opacity: _opacity,
        child: TickerMode(
          enabled: _shown && widget.active,
          child: widget.child,
        ),
      ),
    ),
  );

  @override
  void dispose() {
    _delay?.cancel();
    _opacity.dispose();
    super.dispose();
  }
}

class AmeLinearProgress extends StatelessWidget {
  const AmeLinearProgress({
    this.active = true,
    this.value,
    this.minHeight,
    this.semanticsLabel,
    this.delay = ameProgressDelay,
    super.key,
  });

  final bool active;
  final double? value;
  final double? minHeight;
  final String? semanticsLabel;
  final Duration delay;

  @override
  Widget build(BuildContext context) => AmeProgressVisibility(
    active: active,
    delay: delay,
    child: LinearProgressIndicator(
      value: value,
      minHeight: minHeight,
      semanticsLabel: semanticsLabel,
    ),
  );
}

class AmeCircularProgress extends StatelessWidget {
  const AmeCircularProgress({
    this.active = true,
    this.strokeWidth = 4,
    this.semanticsLabel,
    super.key,
  });

  final bool active;
  final double strokeWidth;
  final String? semanticsLabel;

  @override
  Widget build(BuildContext context) => AmeProgressVisibility(
    active: active,
    child: CircularProgressIndicator(
      strokeWidth: strokeWidth,
      semanticsLabel: semanticsLabel,
    ),
  );
}

class AmeLoadingProgress extends StatelessWidget {
  const AmeLoadingProgress({
    this.active = true,
    this.size = 48,
    this.semanticsLabel = "正在加载",
    this.delay = ameProgressDelay,
    super.key,
  });

  final bool active;
  final double size;
  final String semanticsLabel;
  final Duration delay;

  @override
  Widget build(BuildContext context) => AmeProgressVisibility(
    active: active,
    delay: delay,
    child: AmeLoadingIndicator(size: size, semanticsLabel: semanticsLabel),
  );
}

/// Keeps list geometry and demand mounted while an initial read is pending.
class AmeLoadingContent extends StatelessWidget {
  const AmeLoadingContent({
    required this.isLoading,
    required this.child,
    this.semanticsLabel = "正在加载",
    super.key,
  });

  final bool isLoading;
  final Widget child;
  final String semanticsLabel;

  @override
  Widget build(BuildContext context) => Stack(
    fit: StackFit.expand,
    children: [
      child,
      Center(
        child: AmeLoadingProgress(
          active: isLoading,
          semanticsLabel: semanticsLabel,
        ),
      ),
    ],
  );
}
