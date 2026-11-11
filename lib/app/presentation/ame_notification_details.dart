import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../notifications/ame_notification_controller.dart";
import "../notifications/ame_notification_strings.dart";
import "ame_notification_time.dart";
import "ame_dialog.dart";

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
    return AmeDialog(
      title: AmeNotificationStrings.notificationDetails,
      content: AmeDialogBody(
        children: [
          if (notification.severity == AmeNotificationSeverity.error)
            AmeDialogError(notification.title)
          else
            Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 8,
              children: [
                Text(
                  notification.title,
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ],
            ),
          if (notification.message.isNotEmpty) Text(notification.message),
          if (notification.detail case final detail?) Text(detail),
          if (notification.elapsedStartedAt case final startedAt?
              when !isHistorical) ...[
            Text(
              "已持续 ${formatAmeNotificationElapsed(DateTime.now().difference(startedAt))}",
            ),
          ],
          if (notification.sourcePath case final sourcePath?) ...[
            AmeDialogField(
              label: AmeNotificationStrings.sourcePath,
              child: SelectableText(sourcePath),
            ),
          ],
          if (notification.technicalCode case final technicalCode?) ...[
            AmeDialogField(
              label: AmeNotificationStrings.technicalCode,
              child: SelectableText(technicalCode),
            ),
          ],
          AmeDialogField(
            label: "发生时间",
            child: Text(
              formatAmeNotificationOccurredAt(notification.occurredAt),
            ),
          ),
        ],
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
