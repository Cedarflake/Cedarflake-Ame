import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../notifications/ame_notification_controller.dart";
import "ame_localizations.dart";
import "ame_message_text.dart";
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
      title: context.l10n.notificationDetails,
      content: AmeDialogBody(
        children: [
          if (notification.severity == AmeNotificationSeverity.error)
            AmeDialogError(notification.title.resolve(context.l10n))
          else
            Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 8,
              children: [
                Text(
                  notification.title.resolve(context.l10n),
                  style: Theme.of(context).textTheme.titleMedium,
                ),
              ],
            ),
          if (notification.message.resolve(context.l10n).isNotEmpty)
            Text(notification.message.resolve(context.l10n)),
          if (notification.detail case final detail?)
            Text(detail.resolve(context.l10n)),
          if (notification.elapsedStartedAt case final startedAt?
              when !isHistorical) ...[
            Text(
              context.l10n.elapsedTime(
                formatAmeNotificationElapsed(
                  context.l10n,
                  DateTime.now().difference(startedAt),
                ),
              ),
            ),
          ],
          if (notification.sourcePath case final sourcePath?) ...[
            AmeDialogField(
              label: context.l10n.sourceDirectory,
              child: SelectableText(sourcePath),
            ),
          ],
          if (notification.technicalCode case final technicalCode?) ...[
            AmeDialogField(
              label: context.l10n.technicalCode,
              child: SelectableText(technicalCode),
            ),
          ],
          AmeDialogField(
            label: context.l10n.occurredAt,
            child: Text(
              formatAmeNotificationOccurredAt(
                context.l10n,
                notification.occurredAt,
              ),
            ),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(context.l10n.close),
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
            child: Text(actionLabel.resolve(context.l10n)),
          ),
      ],
    );
  }
}
