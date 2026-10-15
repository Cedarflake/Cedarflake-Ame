import "dart:math" as math;

import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/rendering.dart";

class LibraryGalleryScrollController extends ScrollController {
  LibraryGalleryScrollController({
    super.initialScrollOffset,
    super.onAttach,
    super.onDetach,
  });

  @override
  ScrollPosition createScrollPosition(
    ScrollPhysics physics,
    ScrollContext context,
    ScrollPosition? oldPosition,
  ) => _GalleryScrollPosition(
    physics: physics,
    context: context,
    initialPixels: initialScrollOffset,
    oldPosition: oldPosition,
  );

  void preparePointerScroll({required bool animate}) {
    for (final position in positions.whereType<_GalleryScrollPosition>()) {
      position.animateNextPointerScroll = animate;
    }
  }

  void stopWheelMotion() {
    for (final position in positions.whereType<_GalleryScrollPosition>()) {
      position.stopWheelMotion();
    }
  }
}

class LibraryGalleryScrollMotion extends StatelessWidget {
  const LibraryGalleryScrollMotion({
    required this.controller,
    required this.child,
    this.onPointerSignal,
    super.key,
  });

  final ScrollController controller;
  final Widget child;
  final PointerSignalEventListener? onPointerSignal;

  @override
  Widget build(BuildContext context) {
    final disableAnimations = MediaQuery.disableAnimationsOf(context);
    final scrollController = controller;
    return Listener(
      onPointerDown: (_) {
        if (scrollController is LibraryGalleryScrollController) {
          scrollController.stopWheelMotion();
        }
      },
      onPointerSignal: (event) {
        // Scrollable resolves the signal after its hit-test path is dispatched.
        // It still owns axis modifiers, nested scrolling and event admission.
        if (scrollController is LibraryGalleryScrollController) {
          scrollController.preparePointerScroll(
            animate:
                event is PointerScrollEvent &&
                event.kind == PointerDeviceKind.mouse &&
                !disableAnimations,
          );
        }
        onPointerSignal?.call(event);
      },
      child: child,
    );
  }
}

class _GalleryScrollPosition extends ScrollPositionWithSingleContext {
  _GalleryScrollPosition({
    required super.physics,
    required super.context,
    required super.initialPixels,
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
