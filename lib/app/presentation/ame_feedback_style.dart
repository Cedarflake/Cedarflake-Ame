import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../notifications/ame_notification_controller.dart";

const ameNotificationWidth = 680.0;
const ameNotificationElevation = 3.0;
const ameNotificationRadius = 16.0;
const ameNotificationIconSize = 20.0;
const ameNotificationBottomInset = 24.0;
const ameNotificationDuration = Duration(seconds: 5);
const ameFeedbackCardPadding = EdgeInsets.fromLTRB(20, 14, 12, 12);
const ameViewerNoticeInset = 16.0;

IconData ameNotificationIcon(AmeNotificationSeverity severity) =>
    switch (severity) {
      AmeNotificationSeverity.info => Symbols.info_rounded,
      AmeNotificationSeverity.success => Symbols.check_circle_rounded,
      AmeNotificationSeverity.warning => Symbols.warning_rounded,
      AmeNotificationSeverity.error => Symbols.error_rounded,
    };

Color ameNotificationIconColor(
  ColorScheme colors,
  AmeNotificationSeverity severity,
) => switch (severity) {
  AmeNotificationSeverity.info ||
  AmeNotificationSeverity.success => colors.primary,
  AmeNotificationSeverity.warning => colors.tertiary,
  AmeNotificationSeverity.error => colors.error,
};

class AmeFeedbackAction {
  const AmeFeedbackAction({
    required this.label,
    required this.onPressed,
    this.key,
  });

  final String label;
  final VoidCallback? onPressed;
  final Key? key;

  Widget build() =>
      TextButton(key: key, onPressed: onPressed, child: Text(label));
}

class AmeFeedbackSurface extends StatelessWidget {
  const AmeFeedbackSurface({required this.child, super.key});
  final Widget child;

  @override
  Widget build(BuildContext context) => Material(
    elevation: ameNotificationElevation,
    color: Theme.of(context).colorScheme.surfaceContainerHigh,
    borderRadius: BorderRadius.circular(ameNotificationRadius),
    child: child,
  );
}
