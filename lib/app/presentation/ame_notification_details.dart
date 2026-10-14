import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../notifications/ame_notification_controller.dart";
import "../notifications/ame_notification_strings.dart";
import "ame_notification_time.dart";

Future<void> showAmeNotificationDetails(
  BuildContext context,
  AmeNotificationEntry notification, {
  required ValueChanged<AmeNotificationEntry> onAction,
}) {
  final container = ProviderScope.containerOf(context, listen: false);
  return showDialog<void>(
    context: context,
    builder: (dialogContext) => UncontrolledProviderScope(
      container: container,
      child: _AmeNotificationDetails(
        observed: notification,
        onAction: onAction,
      ),
    ),
  );
}

class _AmeNotificationDetails extends ConsumerWidget {
  const _AmeNotificationDetails({
    required this.observed,
    required this.onAction,
  });

  final AmeNotificationEntry observed;
  final ValueChanged<AmeNotificationEntry> onAction;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(ameNotificationControllerProvider);
    AmeNotificationEntry? retained;
    for (final entry in state.history) {
      if (entry.id == observed.id) {
        retained = entry;
        break;
      }
    }
    final notification = retained ?? observed;
    final isHistorical = retained == null || notification.isResolvedCondition;
    return AlertDialog(
      title: const Text(AmeNotificationStrings.notificationDetails),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                notification.title,
                style: Theme.of(context).textTheme.titleMedium,
              ),
              if (isHistorical) ...[
                const SizedBox(height: 8),
                const Text(AmeNotificationStrings.historicalNotificationDetail),
              ],
              if (notification.message.isNotEmpty) ...[
                const SizedBox(height: 8),
                Text(notification.message),
              ],
              if (notification.detail case final detail?) ...[
                const SizedBox(height: 8),
                Text(detail),
              ],
              if (notification.elapsedStartedAt case final startedAt?
                  when !isHistorical) ...[
                const SizedBox(height: 8),
                Text(
                  "已持续 ${formatAmeNotificationElapsed(DateTime.now().difference(startedAt))}",
                ),
              ],
              if (notification.sourcePath case final sourcePath?) ...[
                const SizedBox(height: 16),
                Text(
                  AmeNotificationStrings.sourcePath,
                  style: Theme.of(context).textTheme.labelMedium,
                ),
                const SizedBox(height: 4),
                SelectableText(sourcePath),
              ],
              if (notification.technicalCode case final technicalCode?) ...[
                const SizedBox(height: 16),
                Text(
                  AmeNotificationStrings.technicalCode,
                  style: Theme.of(context).textTheme.labelMedium,
                ),
                const SizedBox(height: 4),
                SelectableText(technicalCode),
              ],
              const SizedBox(height: 16),
              Text(formatAmeNotificationOccurredAt(notification.occurredAt)),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text(AmeNotificationStrings.close),
        ),
        if (notification.actionLabel case final actionLabel? when !isHistorical)
          FilledButton(
            onPressed: () {
              final admitted = ref
                  .read(ameNotificationControllerProvider.notifier)
                  .admitAction(notification);
              if (admitted == null) {
                return;
              }
              Navigator.of(context).pop();
              onAction(admitted);
            },
            child: Text(actionLabel),
          ),
      ],
    );
  }
}
