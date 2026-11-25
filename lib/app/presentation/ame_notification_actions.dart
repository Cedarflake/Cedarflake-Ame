import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../notifications/ame_notification_controller.dart";

typedef AmeNotificationAction = Future<void> Function(BuildContext context);

final ameNotificationActionsProvider =
    Provider<Map<String, AmeNotificationAction>>((ref) => const {});

bool handleApplicationNotificationAction(
  BuildContext context,
  WidgetRef ref,
  AmeNotificationEntry entry,
) {
  final current = ref
      .read(ameNotificationControllerProvider.notifier)
      .admitAction(entry);
  if (current == null) return false;
  final action = ref.read(ameNotificationActionsProvider)[current.actionId];
  if (action == null) return false;
  unawaited(action(context));
  return true;
}
