import "package:material_ui/material_ui.dart";

import "library_viewer_launch.dart";

/// Fades the viewer's surroundings without fading the travelling image.
class LibraryViewerChrome extends StatelessWidget {
  const LibraryViewerChrome({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final flight = LibraryViewerFlightScope.of(context);
    return FadeTransition(
      opacity: flight?.origin == null
          ? const AlwaysStoppedAnimation(1)
          : flight!.animation,
      alwaysIncludeSemantics: true,
      child: child,
    );
  }
}

class LibraryViewerBackdrop extends StatelessWidget {
  const LibraryViewerBackdrop({
    required this.color,
    required this.child,
    super.key,
  });

  final Color color;
  final Widget child;

  @override
  Widget build(BuildContext context) => Stack(
    fit: StackFit.passthrough,
    clipBehavior: Clip.none,
    children: [
      Positioned.fill(
        child: LibraryViewerChrome(child: ColoredBox(color: color)),
      ),
      child,
    ],
  );
}
