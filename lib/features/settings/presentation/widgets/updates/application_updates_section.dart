import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../../app/presentation/ame_localizations.dart";
import "../../../../../app/window/ame_build_identity.dart";
import "../../../../updates/application/application_update_controller.dart";
import "../../../../updates/application/application_update_state.dart";
import "../settings_section.dart";
import "../diagnostics_settings_row.dart";
import "application_update_dialog.dart";
import "application_update_settings.dart";
import "update_text.dart";

class ApplicationUpdatesSection extends ConsumerWidget {
  const ApplicationUpdatesSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = ref.watch(applicationUpdateControllerProvider);
    final build = ref.watch(ameBuildIdentityProvider);
    final strings = context.l10n;
    return ListenableBuilder(
      listenable: controller,
      builder: (context, _) {
        final state = controller.state;
        final ready = state.phase == ApplicationUpdatePhase.ready;
        final checking = state.phase == ApplicationUpdatePhase.checking;
        final actionLabel = switch (state.phase) {
          ApplicationUpdatePhase.ready => strings.updateRestart,
          _ when state.release != null => strings.updateView,
          ApplicationUpdatePhase.checking => strings.updateChecking,
          _ => strings.updateCheck,
        };
        return SettingsSection(
          title: strings.settingsAbout,
          children: [
            SettingsRow(
              icon: Symbols.info_rounded,
              title: strings.settingsVersion,
              subtitle: Text(build.applicationLabel),
              trailing: FilledButton.tonal(
                key: const Key("application-update-check"),
                onPressed: state.phase == ApplicationUpdatePhase.restarting
                    ? null
                    : () {
                        if (ready) {
                          unawaited(controller.restart());
                          return;
                        }
                        if (state.release == null && !checking) {
                          unawaited(controller.check());
                        }
                        unawaited(
                          showApplicationUpdateDialog(context, controller),
                        );
                      },
                child: Text(actionLabel),
              ),
            ),
            SettingsRow(
              icon: Symbols.update_rounded,
              title: strings.updateSettings,
              subtitle: Text(
                "${updateChannelLabel(strings, state.preferences.channel)} · ${updatePolicyLabel(strings, state.preferences.policy)}",
              ),
              trailing: const Icon(Symbols.chevron_right_rounded),
              onTap: () => showDialog<void>(
                context: context,
                builder: (_) => const ApplicationUpdateSettings(),
              ),
            ),
            SettingsRow(
              key: const Key("open-source-notices-setting"),
              icon: Symbols.code_rounded,
              title: strings.settingsLicenses,
              subtitle: Text(strings.settingsLicensesHelp),
              trailing: const Icon(Symbols.chevron_right_rounded),
              onTap: () => showLicensePage(
                context: context,
                applicationName: "Cedarflake Ame",
                applicationVersion: build.version,
              ),
            ),
            const DiagnosticsSettingsRow(),
          ],
        );
      },
    );
  }
}
