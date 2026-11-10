import "package:flutter/physics.dart";
import "package:material_ui/material_ui.dart";

enum AmeMotion {
  fastSpatial(Springs.standardFastSpatial),
  spatial(Springs.standardDefaultSpatial),
  fastEffects(Springs.standardFastEffects),
  effects(Springs.standardDefaultEffects);

  const AmeMotion(this.spring);

  final SpringDescription spring;

  SpringSimulation simulation(
    AnimationController controller, {
    required bool forward,
  }) => SpringSimulation(
    spring,
    controller.value,
    forward ? 1 : 0,
    controller.velocity,
    snapToEnd: true,
  );

  void animate(AnimationController controller, {required bool forward}) {
    final target = forward ? 1.0 : 0.0;
    if (controller.value == target && !controller.isAnimating) return;
    final transition = simulation(controller, forward: forward);
    if (forward) {
      controller.animateWith(transition);
    } else {
      controller.animateBackWith(transition);
    }
  }
}

class AmeStateTransition extends StatefulWidget {
  const AmeStateTransition({
    required this.active,
    required this.motion,
    required this.builder,
    this.child,
    super.key,
  });

  final bool active;
  final AmeMotion motion;
  final Widget Function(BuildContext, Animation<double>, Widget?) builder;
  final Widget? child;

  @override
  State<AmeStateTransition> createState() => _AmeStateTransitionState();
}

class _AmeStateTransitionState extends State<AmeStateTransition>
    with SingleTickerProviderStateMixin {
  late final _controller = AnimationController(
    vsync: this,
    value: widget.active ? 1 : 0,
  );
  bool get _skipMotion =>
      MediaQuery.disableAnimationsOf(context) ||
      !TickerMode.valuesOf(context).enabled;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_skipMotion) _controller.value = widget.active ? 1 : 0;
  }

  @override
  void didUpdateWidget(covariant AmeStateTransition oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_skipMotion) {
      _controller.value = widget.active ? 1 : 0;
    } else if (widget.active != oldWidget.active ||
        widget.motion != oldWidget.motion) {
      widget.motion.animate(_controller, forward: widget.active);
    }
  }

  @override
  Widget build(BuildContext context) =>
      widget.builder(context, _controller, widget.child);

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }
}
