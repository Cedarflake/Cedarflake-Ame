import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../notifications/ame_notification_controller.dart";
import "ame_localizations.dart";
import "ame_menu.dart";
import "ame_feedback_card.dart";
import "ame_feedback_style.dart";
import "ame_message_text.dart";
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
            context.l10n.notifications,
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
              context.l10n.notificationsEmpty,
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
      labels: [context.l10n.notifications],
      items: menuItems,
      menuWidth: _notificationMenuWidth,
      maximumHeight: _notificationMenuMaximumHeight,
      verticalGap: 6,
      onOpened: onOpened,
      onSelected: onSelected,
      builder: (context, openMenu) {
        final semanticLabel = state.hasUnread
            ? context.l10n.notificationsUnread
            : context.l10n.notificationsRead;
        return Semantics(
          label: semanticLabel,
          button: true,
          child: AmeTooltip(
            message: context.l10n.notifications,
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
    return AmeNotificationLifetime(
      notification: notification,
      onDismiss: onDismiss,
      child: AmeFeedbackCard(
        key: const Key("ame-notification-surface"),
        title: notification.title.resolve(context.l10n),
        message: notification.message.resolve(context.l10n),
        detail: notification.detail?.resolve(context.l10n),
        severity: notification.severity,
        actions: [
          if (notification.actionLabel case final actionLabel?)
            AmeFeedbackAction(
              key: const Key("notification-primary-action"),
              onPressed: () => onAction(notification),
              label: actionLabel.resolve(context.l10n),
            ),
          AmeFeedbackAction(
            key: const Key("notification-dismiss-button"),
            onPressed: () => onDismiss(notification.id),
            label: context.l10n.acknowledge,
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
            ameNotificationIcon(notification.severity),
            color: ameNotificationIconColor(colorScheme, notification.severity),
            size: 18,
          ),
          const SizedBox(width: 10),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  notification.title.resolve(context.l10n),
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
                if (notification.message.resolve(context.l10n).isNotEmpty) ...[
                  const SizedBox(height: 2),
                  Text(
                    notification.message.resolve(context.l10n),
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                ],
                const SizedBox(height: 4),
                Text(
                  notification.isResolvedCondition
                      ? "${context.l10n.notificationHistory} · ${formatAmeNotificationOccurredAt(context.l10n, notification.occurredAt)}"
                      : formatAmeNotificationOccurredAt(
                          context.l10n,
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
