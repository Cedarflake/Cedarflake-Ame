import "package:flutter/material.dart";

import "../notifications/ame_notification_controller.dart";
import "ame_feedback_style.dart";

export "ame_feedback_style.dart" show AmeFeedbackAction;

class AmeFeedbackCard extends StatelessWidget {
  const AmeFeedbackCard({
    required this.title,
    required this.severity,
    required this.actions,
    this.icon,
    this.message,
    this.detail,
    this.progress,
    super.key,
  }) : items = null;

  const AmeFeedbackCard.tasks({
    required this.title,
    required IconData this.icon,
    required Widget this.items,
    required this.actions,
    super.key,
  }) : severity = AmeNotificationSeverity.info,
       message = null,
       detail = null,
       progress = null;

  final String title;
  final AmeNotificationSeverity severity;
  final IconData? icon;
  final String? message;
  final String? detail;
  final List<AmeFeedbackAction> actions;
  final Widget? progress;
  final Widget? items;

  @override
  Widget build(BuildContext context) => AmeFeedbackSurface(
    child: ConstrainedBox(
      constraints: BoxConstraints(
        minWidth: ameNotificationWidth,
        maxWidth: ameNotificationWidth,
        maxHeight: items == null ? double.infinity : 360,
      ),
      child: Padding(
        padding: ameFeedbackCardPadding,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Padding(
                  padding: const EdgeInsets.only(top: 2),
                  child: Icon(
                    icon ?? ameNotificationIcon(severity),
                    color: ameNotificationIconColor(
                      Theme.of(context).colorScheme,
                      severity,
                    ),
                    size: ameNotificationIconSize,
                  ),
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        title,
                        style: Theme.of(context).textTheme.titleSmall,
                      ),
                      if (message case final value? when value.isNotEmpty) ...[
                        const SizedBox(height: 4),
                        Text(
                          value,
                          style: Theme.of(context).textTheme.bodyMedium,
                        ),
                      ],
                      if (detail case final value? when value.isNotEmpty) ...[
                        const SizedBox(height: 4),
                        Text(
                          value,
                          style: Theme.of(context).textTheme.bodySmall
                              ?.copyWith(
                                color: Theme.of(
                                  context,
                                ).colorScheme.onSurfaceVariant,
                              ),
                        ),
                      ],
                    ],
                  ),
                ),
                for (final action in actions) action.build(),
              ],
            ),
            if (progress case final indicator?) ...[
              const SizedBox(height: 10),
              indicator,
            ],
            if (items case final list?) ...[
              const SizedBox(height: 8),
              Flexible(child: list),
            ],
          ],
        ),
      ),
    ),
  );
}
