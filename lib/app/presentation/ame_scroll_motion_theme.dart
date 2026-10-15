import "package:flutter/material.dart";

import "ame_scroll_motion.dart";

/// Carries scroll motion into Material routes captured from the application.
/// Their implicit scroll views receive a separate controller for each route.
class AmeScrollMotionTheme extends InheritedTheme {
  const AmeScrollMotionTheme({required super.child, super.key});

  @override
  Widget wrap(BuildContext context, Widget child) =>
      AmeScrollMotionTheme(child: AmeScrollMotion(child: child));

  @override
  bool updateShouldNotify(AmeScrollMotionTheme oldWidget) => false;
}
