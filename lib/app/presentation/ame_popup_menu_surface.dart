import "package:flutter/material.dart";

/// Keeps an above-anchor popup's bottom edge fixed while Material reveals it.
/// The captured theme only changes surface layout; the popup route still owns
/// input, focus, dismissal and semantics.
class AmePopupMenuSurface extends InheritedTheme {
  const AmePopupMenuSurface({
    required this.resolveHeight,
    required super.child,
    super.key,
  });

  final double? Function() resolveHeight;

  @override
  Widget wrap(BuildContext context, Widget child) {
    final height = resolveHeight();
    if (height == null) return child;
    return SizedBox(
      height: height,
      child: Align(
        alignment: AlignmentDirectional.bottomEnd,
        widthFactor: 1,
        child: child,
      ),
    );
  }

  @override
  bool updateShouldNotify(AmePopupMenuSurface oldWidget) => false;
}
