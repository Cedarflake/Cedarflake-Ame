import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../notifications/ame_notification_controller.dart";
import "../notifications/ame_notification_strings.dart";
import "ame_menu.dart";
import "ame_feedback_card.dart";
import "ame_notification_lifetime.dart";
import "ame_overlay_semantics.dart";
import "ame_notification_time.dart";

export "ame_notification_details.dart" show showAmeNotificationDetails;

const _notificationMenuWidth = 360.0;
const _notificationMenuMaximumHeight = 520.0;

class AmeNotificationHistoryButton extends StatelessWidget {
  const AmeNotificationHistoryButton({
    required this.state,
    required this.onOpened,
    required this.onSelected,
    super.key,
  });

  final AmeNotificationState state;
  final VoidCallback onOpened;
  final ValueChanged<AmeNotificationEntry> onSelected;

  @override
  Widget build(BuildContext context) {
    final onSurface = Theme.of(context).colorScheme.onSurface;
    final menuItems = <PopupMenuEntry<AmeNotificationEntry>>[
      PopupMenuItem<AmeNotificationEntry>(
        enabled: false,
        padding: EdgeInsets.zero,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
          child: Text(
            AmeNotificationStrings.notifications,
            style: TextStyle(color: onSurface, fontWeight: FontWeight.w600),
          ),
        ),
      ),
      const PopupMenuDivider(height: 1),
      if (state.history.isEmpty)
        PopupMenuItem<AmeNotificationEntry>(
          enabled: false,
          padding: EdgeInsets.zero,
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: Text(
              AmeNotificationStrings.noNotifications,
              style: TextStyle(color: onSurface),
            ),
          ),
        )
      else
        for (final notification in state.history)
          PopupMenuItem<AmeNotificationEntry>(
            key: Key("notification-history-item-${notification.id}"),
            value: notification,
            child: _NotificationHistoryItem(notification: notification),
          ),
    ];
    return AmePopupMenuButton<AmeNotificationEntry>(
      labels: const [AmeNotificationStrings.notifications],
      items: menuItems,
      menuWidth: _notificationMenuWidth,
      maximumHeight: _notificationMenuMaximumHeight,
      verticalGap: 6,
      onOpened: onOpened,
      onSelected: onSelected,
      builder: (context, openMenu) {
        final semanticLabel = state.hasUnread ? "通知，有未读消息" : "通知，无未读消息";
        return Semantics(
          label: semanticLabel,
          button: true,
          child: AmeTooltip(
            message: AmeNotificationStrings.notifications,
            child: IconButton(
              key: const Key("notification-history-button"),
              onPressed: openMenu,
              icon: Icon(
                state.hasUnread
                    ? Symbols.notifications_unread_rounded
                    : Symbols.notifications_rounded,
                key: Key(
                  state.hasUnread
                      ? "notification-unread-icon"
                      : "notification-read-icon",
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

class AmeNotificationSurface extends StatelessWidget {
  const AmeNotificationSurface({
    required this.notification,
    required this.onAction,
    required this.onDismiss,
    super.key,
  });

  final AmeNotificationEntry notification;
  final ValueChanged<AmeNotificationEntry> onAction;
  final ValueChanged<String> onDismiss;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return AmeNotificationLifetime(
      notification: notification,
      onDismiss: onDismiss,
      child: AmeFeedbackCard(
        key: const Key("ame-notification-surface"),
        leading: Icon(
          _severityIcon(notification.severity),
          color: _severityColor(colorScheme, notification.severity),
          size: 20,
        ),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              notification.title,
              style: Theme.of(context).textTheme.titleSmall,
            ),
            if (notification.message.isNotEmpty) ...[
              const SizedBox(height: 4),
              Text(notification.message),
            ],
            if (notification.detail case final detail?) ...[
              const SizedBox(height: 4),
              Text(
                detail,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ],
        ),
        actions: [
          if (notification.actionLabel case final actionLabel?)
            TextButton(
              key: const Key("notification-primary-action"),
              onPressed: () => onAction(notification),
              child: Text(actionLabel),
            ),
          TextButton(
            key: const Key("notification-dismiss-button"),
            onPressed: () => onDismiss(notification.id),
            child: const Text(AmeNotificationStrings.acknowledge),
          ),
        ],
      ),
    );
  }
}

class _NotificationHistoryItem extends StatelessWidget {
  const _NotificationHistoryItem({required this.notification});

  final AmeNotificationEntry notification;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 6),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(
            _severityIcon(notification.severity),
            color: _severityColor(colorScheme, notification.severity),
            size: 18,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  notification.title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                if (notification.message.isNotEmpty) ...[
                  const SizedBox(height: 2),
                  Text(
                    notification.message,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
                const SizedBox(height: 4),
                Text(
                  notification.isResolvedCondition
                      ? "${AmeNotificationStrings.historicalNotification} · ${formatAmeNotificationOccurredAt(notification.occurredAt)}"
                      : formatAmeNotificationOccurredAt(
                          notification.occurredAt,
                        ),
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
          if (notification.isUnread)
            Container(
              key: const Key("notification-history-unread-dot"),
              width: 6,
              height: 6,
              margin: const EdgeInsets.only(top: 5, left: 8),
              decoration: BoxDecoration(
                color: colorScheme.primary,
                shape: BoxShape.circle,
              ),
            ),
        ],
      ),
    );
  }
}

IconData _severityIcon(AmeNotificationSeverity severity) {
  return switch (severity) {
    AmeNotificationSeverity.info => Symbols.info_rounded,
    AmeNotificationSeverity.success => Symbols.check_circle_rounded,
    AmeNotificationSeverity.warning => Symbols.warning_rounded,
    AmeNotificationSeverity.error => Symbols.error_rounded,
  };
}

Color _severityColor(
  ColorScheme colorScheme,
  AmeNotificationSeverity severity,
) {
  return switch (severity) {
    AmeNotificationSeverity.info => colorScheme.primary,
    AmeNotificationSeverity.success => colorScheme.primary,
    AmeNotificationSeverity.warning => colorScheme.tertiary,
    AmeNotificationSeverity.error => colorScheme.error,
  };
}
