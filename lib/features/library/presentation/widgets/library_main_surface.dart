import "package:flutter/material.dart";

class LibraryMainSurface extends StatelessWidget {
  const LibraryMainSurface({required this.child, super.key});

  static const borderRadius = BorderRadius.only(topLeft: Radius.circular(18));

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surfaceContainerLowest,
        borderRadius: borderRadius,
      ),
      child: ClipRRect(
        borderRadius: borderRadius,
        child: Material(
          key: const Key("library-main-surface"),
          type: MaterialType.transparency,
          child: child,
        ),
      ),
    );
  }
}
