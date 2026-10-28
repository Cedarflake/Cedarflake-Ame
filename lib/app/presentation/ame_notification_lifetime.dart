import "dart:async";

import "package:flutter/widgets.dart";

import "../notifications/ame_notification_controller.dart";
import "ame_feedback_style.dart";

class AmeNotificationLifetime extends StatefulWidget {
  const AmeNotificationLifetime({
    required this.notification,
    required this.onDismiss,
    required this.child,
    super.key,
  });

  final AmeNotificationEntry notification;
  final ValueChanged<String> onDismiss;
  final Widget child;

  @override
  State<AmeNotificationLifetime> createState() =>
      _AmeNotificationLifetimeState();
}

class _AmeNotificationLifetimeState extends State<AmeNotificationLifetime> {
  Timer? _dismissTimer;

  @override
  void initState() {
    super.initState();
    _scheduleDismiss();
  }

  @override
  void didUpdateWidget(AmeNotificationLifetime oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.notification.id != widget.notification.id ||
        oldWidget.notification.isPersistent !=
            widget.notification.isPersistent) {
      _scheduleDismiss();
    }
  }

  @override
  void dispose() {
    _dismissTimer?.cancel();
    super.dispose();
  }

  void _scheduleDismiss() {
    _dismissTimer?.cancel();
    if (widget.notification.isPersistent) return;
    _dismissTimer = Timer(
      ameNotificationDuration,
      () => widget.onDismiss(widget.notification.id),
    );
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
