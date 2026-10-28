import "package:flutter/material.dart";

import "../notifications/ame_notification_controller.dart";
import "../notifications/ame_notification_strings.dart";
import "ame_feedback_card.dart";
import "ame_feedback_style.dart";

export "../notifications/ame_notification_controller.dart"
    show AmeNotificationSeverity;

void showAmeTransientFeedback(
  BuildContext context, {
  required String message,
  required AmeNotificationSeverity severity,
}) {
  late ScaffoldFeatureController<SnackBar, SnackBarClosedReason> controller;
  controller = ScaffoldMessenger.of(context).showSnackBar(
    SnackBar(
      padding: EdgeInsets.zero,
      backgroundColor: Colors.transparent,
      elevation: 0,
      clipBehavior: Clip.none,
      duration: ameNotificationDuration,
      content: AmeFeedbackCard(
        title: message,
        severity: severity,
        actions: [
          AmeFeedbackAction(
            label: AmeNotificationStrings.acknowledge,
            onPressed: () => controller.close(),
          ),
        ],
      ),
    ),
  );
}
