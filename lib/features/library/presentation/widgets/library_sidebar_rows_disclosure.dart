import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_motion.dart";

class LibrarySidebarRowsDisclosure extends StatelessWidget {
  const LibrarySidebarRowsDisclosure({
    required this.expanded,
    required this.child,
    super.key,
  });

  final bool expanded;
  final Widget child;

  @override
  Widget build(BuildContext context) => AmeStateTransition(
    active: expanded,
    motion: AmeMotion.fastSpatial,
    child: child,
    builder: (context, progress, child) => AmeStateTransition(
      active: expanded,
      motion: AmeMotion.fastEffects,
      child: child,
      builder: (context, opacity, child) => AnimatedBuilder(
        animation: Listenable.merge([progress, opacity]),
        child: child,
        builder: (context, child) => LibrarySidebarRowsVisibility(
          expanded: expanded,
          visible: expanded || !progress.isDismissed || !opacity.isDismissed,
          progress: progress,
          opacity: opacity,
          child: child!,
        ),
      ),
    ),
  );
}

class LibrarySidebarRowsVisibility extends InheritedWidget {
  const LibrarySidebarRowsVisibility({
    required this.expanded,
    required this.visible,
    required this.progress,
    required this.opacity,
    required super.child,
    super.key,
  });

  final bool expanded;
  final bool visible;
  final Animation<double> progress;
  final Animation<double> opacity;

  static LibrarySidebarRowsVisibility? maybeOf(BuildContext context) => context
      .dependOnInheritedWidgetOfExactType<LibrarySidebarRowsVisibility>();

  Widget wrapRow(Widget child) => SizeTransition(
    sizeFactor: progress,
    alignment: Alignment.topCenter,
    child: FadeTransition(
      opacity: opacity,
      child: IgnorePointer(
        ignoring: !expanded,
        child: ExcludeFocus(
          excluding: !expanded,
          child: ExcludeSemantics(
            excluding: !expanded,
            child: TickerMode(enabled: expanded, child: child),
          ),
        ),
      ),
    ),
  );

  @override
  bool updateShouldNotify(LibrarySidebarRowsVisibility oldWidget) =>
      expanded != oldWidget.expanded ||
      visible != oldWidget.visible ||
      progress != oldWidget.progress ||
      opacity != oldWidget.opacity;
}
