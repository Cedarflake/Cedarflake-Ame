import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_motion.dart";

class LibraryViewerCardOverlayScope extends InheritedWidget {
  const LibraryViewerCardOverlayScope({
    required this.entry,
    required this.viewerOpen,
    required this.returned,
    required super.child,
    super.key,
  });

  final Animation<double> entry;
  final bool viewerOpen;
  final bool returned;

  @override
  bool updateShouldNotify(LibraryViewerCardOverlayScope oldWidget) =>
      entry != oldWidget.entry ||
      viewerOpen != oldWidget.viewerOpen ||
      returned != oldWidget.returned;
}

/// Card badges leave with entry and return only after the image has landed.
class LibraryViewerCardOverlays extends StatelessWidget {
  const LibraryViewerCardOverlays({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<LibraryViewerCardOverlayScope>();
    if (scope == null) return child;
    return AmeStateTransition(
      active: scope.returned,
      motion: AmeMotion.fastEffects,
      child: child,
      builder: (context, reveal, child) => IgnorePointer(
        ignoring: !scope.returned,
        child: ExcludeSemantics(
          excluding: !scope.returned,
          child: FadeTransition(
            opacity: scope.viewerOpen ? ReverseAnimation(scope.entry) : reveal,
            child: child,
          ),
        ),
      ),
    );
  }
}
