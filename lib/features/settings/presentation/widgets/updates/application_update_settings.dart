import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../../app/messages/ame_message.dart";
import "../../../../../app/notifications/ame_notification_controller.dart";
import "../../../../../app/presentation/ame_choice.dart";
import "../../../../../app/presentation/ame_localizations.dart";
import "../../../../../app/presentation/ame_workbench_surfaces.dart";
import "../../../../updates/application/application_update_controller.dart";
import "../../../../updates/domain/application_update.dart";
import "../settings_section.dart";
import "update_text.dart";

class ApplicationUpdateSettings extends ConsumerWidget {
  const ApplicationUpdateSettings({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = ref.watch(applicationUpdateControllerProvider);
    final strings = context.l10n;
    return Scaffold(
      backgroundColor: AmeWorkbenchSurfaces.of(context).content,
      appBar: AppBar(title: Text(strings.updateSettings)),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 980),
            child: ListenableBuilder(
              listenable: controller,
              builder: (context, _) {
                final preferences = controller.state.preferences;
                return SettingsSection(
                  children: [
                    SettingsRow(
                      icon: Symbols.update_rounded,
                      title: strings.updatePolicy,
                      trailing: AmeChoice<ApplicationUpdatePolicy>.menu(
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
                    SettingsRow(
                      icon: Symbols.new_releases_rounded,
                      title: strings.updateChannel,
                      trailing: AmeChoice<ApplicationUpdateChannel>.menu(
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
          ),
        ),
      ),
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
