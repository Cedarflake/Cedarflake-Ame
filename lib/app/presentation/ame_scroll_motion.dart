import "dart:math" as math;

import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/rendering.dart";

class AmeScrollController extends ScrollController {
  AmeScrollController({
    super.initialScrollOffset,
    super.keepScrollOffset,
    super.debugLabel,
    super.onAttach,
    super.onDetach,
  });

  @override
  ScrollPosition createScrollPosition(
    ScrollPhysics physics,
    ScrollContext context,
    ScrollPosition? oldPosition,
  ) => _AmeScrollPosition(
    physics: physics,
    context: context,
    initialPixels: initialScrollOffset,
    keepScrollOffset: keepScrollOffset,
    debugLabel: debugLabel,
    oldPosition: oldPosition,
  );

  void preparePointerScroll({required bool animate}) {
    for (final position in positions.whereType<_AmeScrollPosition>()) {
      position.animateNextPointerScroll = animate;
    }
  }

  void stopWheelMotion() {
    for (final position in positions.whereType<_AmeScrollPosition>()) {
      position.stopWheelMotion();
    }
  }
}

/// Wrap one native scroll view. Omit [controller] to own its primary controller.
class AmeScrollMotion extends StatefulWidget {
  const AmeScrollMotion({
    required this.child,
    this.controller,
    this.scrollDirection = Axis.vertical,
    this.onPointerSignal,
    super.key,
  });

  final ScrollController? controller;
  final Axis scrollDirection;
  final Widget child;
  final PointerSignalEventListener? onPointerSignal;

  @override
  State<AmeScrollMotion> createState() => _AmeScrollMotionState();
}

class _AmeScrollMotionState extends State<AmeScrollMotion> {
  AmeScrollController? _ownedController;

  ScrollController get _controller =>
      widget.controller ?? (_ownedController ??= AmeScrollController());

  @override
  void dispose() {
    _ownedController?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final disableAnimations = MediaQuery.disableAnimationsOf(context);
    final scrollController = _controller;
    return Listener(
      onPointerDown: (_) {
        if (scrollController is AmeScrollController) {
          scrollController.stopWheelMotion();
        }
      },
      onPointerSignal: (event) {
        // Scrollable resolves the signal after its hit-test path is dispatched.
        // It still owns axis modifiers, nested scrolling and event admission.
        if (scrollController is AmeScrollController) {
          scrollController.preparePointerScroll(
            animate:
                event is PointerScrollEvent &&
                event.kind == PointerDeviceKind.mouse &&
                !disableAnimations,
          );
        }
        _routeHorizontalWheel(event);
        widget.onPointerSignal?.call(event);
      },
      child: PrimaryScrollController(
        controller: scrollController,
        scrollDirection: widget.scrollDirection,
        automaticallyInheritForPlatforms: const {...TargetPlatform.values},
        child: widget.child,
      ),
    );
  }

  void _routeHorizontalWheel(PointerSignalEvent event) {
    if (widget.scrollDirection != Axis.horizontal ||
        event is! PointerScrollEvent ||
        event.kind != PointerDeviceKind.mouse ||
        event.scrollDelta.dx != 0 ||
        event.scrollDelta.dy == 0 ||
        _controller.positions.length != 1) {
      return;
    }
    final position = _controller.position;
    if (!position.hasContentDimensions ||
        !position.physics.shouldAcceptUserOffset(position)) {
      return;
    }
    final delta = axisDirectionIsReversed(position.axisDirection)
        ? -event.scrollDelta.dy
        : event.scrollDelta.dy;
    final target = (position.pixels + delta).clamp(
      position.minScrollExtent,
      position.maxScrollExtent,
    );
    if (target == position.pixels) return;
    // The inner Scrollable has first claim on horizontal and Shift-wheel input.
    // At an edge, leave the event available to an enclosing scroll view.
    GestureBinding.instance.pointerSignalResolver.register(event, (_) {
      if (mounted && _controller.positions.contains(position)) {
        position.pointerScroll(delta);
      }
    });
  }
}

class _AmeScrollPosition extends ScrollPositionWithSingleContext {
  _AmeScrollPosition({
    required super.physics,
    required super.context,
    required super.initialPixels,
    super.keepScrollOffset,
    super.debugLabel,
    super.oldPosition,
  });

  bool animateNextPointerScroll = false;

  void stopWheelMotion() {
    if (activity is _WheelScrollActivity) goIdle();
  }

  @override
  void correctPixels(double value) {
    super.correctPixels(value);
    stopWheelMotion();
  }

  @override
  void correctBy(double correction) {
    super.correctBy(correction);
    stopWheelMotion();
  }

  @override
  void pointerScroll(double delta) {
    final animate = animateNextPointerScroll;
    animateNextPointerScroll = false;
    if (!animate || delta == 0 || !hasContentDimensions) {
      super.pointerScroll(delta);
      return;
    }

    final previous = activity;
    var origin = pixels;
    if (previous is _WheelScrollActivity && previous.direction == delta.sign) {
      origin = previous.target;
    }
    final target = (origin + delta)
        .clamp(minScrollExtent, maxScrollExtent)
        .toDouble();
    final distance = target - pixels;
    final tail = math.min(distance.abs() * 0.25, 24.0);
    if (tail < 0.5) {
      super.pointerScroll(distance);
      return;
    }

    // Keep only a short tail in the framework activity. Repeated wheel input
    // preserves its total distance; reversal drops the previous direction's tail.
    final immediatePosition = target - distance.sign * tail;
    final previousPixels = pixels;
    updateUserScrollDirection(
      delta < 0 ? ScrollDirection.forward : ScrollDirection.reverse,
    );
    beginActivity(
      _WheelScrollActivity(
        this,
        from: immediatePosition,
        target: target,
        direction: delta.sign,
        vsync: context.vsync,
      ),
    );
    forcePixels(immediatePosition);
    didUpdateScrollPositionBy(pixels - previousPixels);
  }
}

class _WheelScrollActivity extends DrivenScrollActivity {
  _WheelScrollActivity(
    super.delegate, {
    required super.from,
    required this.target,
    required this.direction,
    required super.vsync,
  }) : super(
         to: target,
         duration: const Duration(milliseconds: 90),
         curve: Curves.easeOutCubic,
       );

  final double target;
  final double direction;

  @override
  bool get shouldIgnorePointer => false;

  // Layout corrections own the new position; an old tail cannot undo them.
  @override
  void applyNewDimensions() => delegate.goIdle();

  @override
  void resetActivity() => delegate.goIdle();
}
