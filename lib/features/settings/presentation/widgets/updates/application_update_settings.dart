import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../../app/messages/ame_message.dart";
import "../../../../../app/notifications/ame_notification_controller.dart";
import "../../../../../app/presentation/ame_choice.dart";
import "../../../../../app/presentation/ame_dialog.dart";
import "../../../../../app/presentation/ame_localizations.dart";
import "../../../../updates/application/application_update_controller.dart";
import "../../../../updates/domain/application_update.dart";
import "update_text.dart";

class ApplicationUpdateSettings extends ConsumerWidget {
  const ApplicationUpdateSettings({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = ref.watch(applicationUpdateControllerProvider);
    final strings = context.l10n;
    return AmeDialog(
      title: strings.updateSettings,
      content: ListenableBuilder(
        listenable: controller,
        builder: (context, _) {
          final preferences = controller.state.preferences;
          return AmeDialogBody(
            children: [
              AmeDialogField(
                label: strings.updatePolicy,
                child: AmeChoice<ApplicationUpdatePolicy>.menu(
                  value: preferences.policy,
                  entries: [
                    for (final policy in ApplicationUpdatePolicy.values)
                      AmeChoiceEntry(
                        value: policy,
                        label: updatePolicyLabel(strings, policy),
                      ),
                  ],
                  onSelected: (policy) {
                    if (policy != null) {
                      unawaited(
                        _save(
                          ref,
                          controller,
                          preferences.copyWith(policy: policy),
                        ),
                      );
                    }
                  },
                ),
              ),
              AmeDialogField(
                label: strings.updateChannel,
                child: AmeChoice<ApplicationUpdateChannel>.menu(
                  value: preferences.channel,
                  entries: [
                    for (final channel in ApplicationUpdateChannel.values)
                      AmeChoiceEntry(
                        value: channel,
                        label: updateChannelLabel(strings, channel),
                      ),
                  ],
                  onSelected: (channel) {
                    if (channel != null) {
                      unawaited(
                        _save(
                          ref,
                          controller,
                          preferences.copyWith(channel: channel),
                        ),
                      );
                    }
                  },
                ),
              ),
            ],
          );
        },
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(strings.close),
        ),
      ],
    );
  }

  Future<void> _save(
    WidgetRef ref,
    ApplicationUpdateController controller,
    ApplicationUpdatePreferences preferences,
  ) async {
    final notifications = ref.read(ameNotificationControllerProvider.notifier);
    try {
      await controller.setPreferences(preferences);
    } on Object catch (error) {
      notifications.publish(
        AmeNotificationDraft(
          title: AmeLabel.settingsSaveFailed,
          message: AmeLabel.operationFailedDetails,
          detail: AmeRawMessage(error.toString()),
          severity: AmeNotificationSeverity.error,
          isPersistent: true,
        ),
      );
    }
  }
}
