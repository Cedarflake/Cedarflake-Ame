import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_motion.dart";
import "library_viewer_launch.dart";

class LibraryViewerFade extends StatelessWidget {
  const LibraryViewerFade({
    required this.animation,
    required this.child,
    super.key,
  });

  static const enterMotion = AmeMotion.effects;
  static const exitMotion = AmeMotion.fastEffects;

  final Animation<double> animation;
  final Widget child;

  @override
  Widget build(BuildContext context) => FadeTransition(
    opacity: animation,
    // Overlay traversal anchors remain reachable during transparent frames.
    alwaysIncludeSemantics: true,
    child: child,
  );
}

class LibraryViewerRoute extends PageRouteBuilder<void> {
  LibraryViewerRoute({
    required BuildContext context,
    required WidgetBuilder builder,
    this.origin,
  }) : _initialReducedMotion = MediaQuery.disableAnimationsOf(context),
       super(
         transitionDuration: Duration.zero,
         reverseTransitionDuration: Duration.zero,
         // Page routes are briefly offstage for Hero measurement. Keep the
         // slider's portal inside that lifecycle so its traversal parent and
         // overlay child enter and retire together.
         pageBuilder: (context, animation, secondaryAnimation) =>
             Overlay.wrap(child: Builder(builder: builder)),
       );

  final bool _initialReducedMotion;
  final LibraryViewerOrigin? origin;

  @override
  Simulation? createSimulation({required bool forward}) {
    final reducedMotion = navigator == null
        ? _initialReducedMotion
        : MediaQuery.disableAnimationsOf(navigator!.context);
    if (reducedMotion) return null;
    final AmeMotion motion;
    if (origin == null) {
      motion = forward
          ? LibraryViewerFade.enterMotion
          : LibraryViewerFade.exitMotion;
    } else {
      motion = forward ? AmeMotion.spatial : AmeMotion.fastSpatial;
    }
    return motion.simulation(controller!, forward: forward);
  }

  void _finishEntry(PointerEvent event) {
    if (animation?.status == AnimationStatus.forward) controller?.value = 1;
  }

  @override
  Widget buildTransitions(
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) => Listener(
    onPointerDown: _finishEntry,
    onPointerSignal: _finishEntry,
    child: LibraryViewerFade(
      animation: origin == null ? animation : const AlwaysStoppedAnimation(1),
      child: LibraryViewerFlightScope(
        origin: origin,
        animation: animation,
        child: child,
      ),
    ),
  );
}
