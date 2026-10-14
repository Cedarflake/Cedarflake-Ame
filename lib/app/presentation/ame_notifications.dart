import "dart:async";

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../notifications/ame_notification_controller.dart";
import "../notifications/ame_notification_strings.dart";
import "ame_menu.dart";
import "ame_overlay_semantics.dart";
import "ame_theme.dart";
import "ame_notification_time.dart";

export "ame_notification_details.dart" show showAmeNotificationDetails;

const _notificationMenuWidth = 360.0;
const _notificationMenuMaximumHeight = 520.0;
const _transientNotificationDuration = Duration(seconds: 5);

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

class AmeNotificationSurface extends StatefulWidget {
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
  State<AmeNotificationSurface> createState() => _AmeNotificationSurfaceState();
}

class _AmeNotificationSurfaceState extends State<AmeNotificationSurface> {
  Timer? _dismissTimer;

  @override
  void initState() {
    super.initState();
    _scheduleDismiss();
  }

  @override
  void didUpdateWidget(AmeNotificationSurface oldWidget) {
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
    if (widget.notification.isPersistent) {
      return;
    }
    _dismissTimer = Timer(
      _transientNotificationDuration,
      () => widget.onDismiss(widget.notification.id),
    );
  }

  @override
  Widget build(BuildContext context) {
    final notification = widget.notification;
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      key: const Key("ame-notification-surface"),
      elevation: ameNotificationElevation,
      color: colorScheme.surfaceContainerHigh,
      borderRadius: BorderRadius.circular(ameNotificationRadius),
      child: ConstrainedBox(
        constraints: const BoxConstraints.tightFor(width: ameNotificationWidth),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 14, 12, 12),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Icon(
                  _severityIcon(notification.severity),
                  color: _severityColor(colorScheme, notification.severity),
                  size: 20,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
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
              ),
              if (notification.actionLabel case final actionLabel?)
                TextButton(
                  key: const Key("notification-primary-action"),
                  onPressed: () => widget.onAction(notification),
                  child: Text(actionLabel),
                ),
              TextButton(
                key: const Key("notification-dismiss-button"),
                onPressed: () => widget.onDismiss(notification.id),
                child: const Text(AmeNotificationStrings.acknowledge),
              ),
            ],
          ),
        ),
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
