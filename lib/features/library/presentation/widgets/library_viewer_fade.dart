import "package:flutter/material.dart";

class LibraryViewerFade extends StatelessWidget {
  const LibraryViewerFade({
    required this.animation,
    required this.child,
    super.key,
  });

  static const enterDuration = Duration(milliseconds: 180);
  static const exitDuration = Duration(milliseconds: 100);

  final Animation<double> animation;
  final Widget child;

  @override
  Widget build(BuildContext context) => FadeTransition(
    opacity: animation.drive(CurveTween(curve: Curves.easeOut)),
    // Overlay traversal anchors remain reachable during transparent frames.
    alwaysIncludeSemantics: true,
    child: child,
  );
}

class LibraryViewerRoute extends PageRouteBuilder<void> {
  LibraryViewerRoute({
    required BuildContext context,
    required WidgetBuilder builder,
  }) : super(
         transitionDuration: MediaQuery.disableAnimationsOf(context)
             ? Duration.zero
             : LibraryViewerFade.enterDuration,
         reverseTransitionDuration: MediaQuery.disableAnimationsOf(context)
             ? Duration.zero
             : LibraryViewerFade.exitDuration,
         pageBuilder: (context, animation, secondaryAnimation) =>
             builder(context),
       );

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
    child: LibraryViewerFade(animation: animation, child: child),
  );
}
