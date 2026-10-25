import "package:flutter/material.dart";
import "package:flutter/rendering.dart";

class LibrarySidebarMotion extends InheritedWidget {
  const LibrarySidebarMotion({
    required this.progress,
    required this.sidebarWidth,
    required this.expanded,
    required super.child,
    super.key,
  });

  final Animation<double> progress;
  final double sidebarWidth;
  final bool expanded;

  double get leadingOffset =>
      sidebarWidth * (progress.value - (expanded ? 1 : 0));

  @override
  bool updateShouldNotify(LibrarySidebarMotion oldWidget) =>
      progress != oldWidget.progress ||
      sidebarWidth != oldWidget.sidebarWidth ||
      expanded != oldWidget.expanded;
}

class LibrarySidebarLeading extends StatelessWidget {
  const LibrarySidebarLeading({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final motion = context
        .dependOnInheritedWidgetOfExactType<LibrarySidebarMotion>();
    return AnimatedBuilder(
      animation: motion?.progress ?? const AlwaysStoppedAnimation(1.0),
      child: child,
      builder: (context, child) => Transform.translate(
        offset: Offset(motion?.leadingOffset ?? 0, 0),
        child: child,
      ),
    );
  }
}

/// Moves content with the sidebar while retaining its destination layout width.
class LibrarySidebarContent extends StatelessWidget {
  const LibrarySidebarContent({
    required this.child,
    this.alignment = Alignment.topLeft,
    super.key,
  });

  final Widget child;
  final Alignment alignment;

  @override
  Widget build(BuildContext context) {
    final motion = context
        .dependOnInheritedWidgetOfExactType<LibrarySidebarMotion>();
    return LayoutBuilder(
      builder: (context, constraints) => SizedBox(
        width: constraints.maxWidth,
        child: AnimatedBuilder(
          animation: motion?.progress ?? const AlwaysStoppedAnimation(1.0),
          child: child,
          builder: (context, content) {
            final delta = motion?.leadingOffset ?? 0;
            final visibleWidth = (constraints.maxWidth - delta).clamp(
              0.0,
              double.infinity,
            );
            return OverflowBox(
              alignment: Alignment.topRight,
              fit: OverflowBoxFit.deferToChild,
              minWidth: visibleWidth,
              maxWidth: visibleWidth,
              child: SizedBox(
                width: visibleWidth,
                child: ClipRect(
                  child: OverflowBox(
                    alignment: alignment,
                    fit: OverflowBoxFit.deferToChild,
                    minWidth: constraints.maxWidth,
                    maxWidth: constraints.maxWidth,
                    child: content,
                  ),
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}
