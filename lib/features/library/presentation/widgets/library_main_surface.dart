import "package:flutter/material.dart";

class LibraryMainSurface extends StatelessWidget {
  const LibraryMainSurface({required this.child, super.key});

  static const borderRadius = BorderRadius.only(topLeft: Radius.circular(18));

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return Material(
      key: const Key("library-main-surface"),
      color: Theme.of(context).colorScheme.surfaceContainerLowest,
      shape: const RoundedRectangleBorder(borderRadius: borderRadius),
      clipBehavior: Clip.antiAlias,
      child: child,
    );
  }
}
