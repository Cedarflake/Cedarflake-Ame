import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../diagnostics/application/diagnostics_controller.dart";
import "../../../diagnostics/application/ports/diagnostic_export.dart";
import "settings_section.dart";

class DiagnosticsSettingsRow extends ConsumerWidget {
  const DiagnosticsSettingsRow({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final controller = ref.read(diagnosticsControllerProvider);
    final state = ref.watch(diagnosticExportStateProvider);
    final strings = context.l10n;
    return SettingsRow(
      icon: Symbols.bug_report_rounded,
      title: strings.diagnosticsTitle,
      subtitle: Text(strings.diagnosticsHelp),
      trailing: FilledButton.tonal(
        key: const Key("diagnostics-export"),
        onPressed: state.busy
            ? null
            : () => unawaited(
                controller.export(
                  DiagnosticSaveLabels(
                    type: strings.diagnosticsFileType,
                    save: strings.diagnosticsExport,
                  ),
                ),
              ),
        child: Text(strings.diagnosticsExport),
      ),
    );
  }
}
