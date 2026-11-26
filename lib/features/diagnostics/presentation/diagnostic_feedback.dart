import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/messages/ame_message.dart";
import "../../../app/notifications/ame_notification_controller.dart";
import "../../../app/presentation/ame_feedback_card.dart";
import "../../../app/presentation/ame_localizations.dart";
import "../../../app/presentation/ame_notifications.dart";
import "../../../app/presentation/ame_progress.dart";
import "../application/diagnostics_controller.dart";
import "../application/ports/diagnostic_export.dart";
import "../domain/diagnostic_report.dart";

DiagnosticSaveLabels diagnosticSaveLabels(AppLocalizations strings) =>
    DiagnosticSaveLabels(
      type: strings.diagnosticsFileType,
      save: strings.diagnosticsExport,
    );

Widget? buildDiagnosticFeedback(BuildContext context, WidgetRef ref) {
  final controller = ref.read(diagnosticsControllerProvider);
  final state = ref.watch(diagnosticExportStateProvider);
  if (state.phase == DiagnosticExportPhase.idle ||
      state.phase == DiagnosticExportPhase.choosing) {
    return null;
  }
  return DiagnosticFeedback(controller: controller);
}

class DiagnosticFeedback extends StatelessWidget {
  const DiagnosticFeedback({required this.controller, super.key});
  final DiagnosticsController controller;

  @override
  Widget build(BuildContext context) {
    final state = controller.state;
    final strings = context.l10n;
    if (state.busy) {
      return AmeProgressVisibility(
        interactive: true,
        child: AmeFeedbackCard(
          title: state.phase == DiagnosticExportPhase.stopping
              ? strings.stopping
              : strings.diagnosticsCollecting,
          severity: AmeNotificationSeverity.info,
          progress: const AmeLinearProgress(delay: Duration.zero),
          actions: [
            AmeFeedbackAction(
              label: strings.cancel,
              onPressed: state.phase == DiagnosticExportPhase.stopping
                  ? null
                  : controller.cancel,
            ),
          ],
        ),
      );
    }
    final failed = state.phase == DiagnosticExportPhase.failed;
    final nativeCode = state.failure?.nativeCode?.label;
    final message = switch ((state.phase, state.failure?.kind)) {
      (DiagnosticExportPhase.failed, DiagnosticFailureKind.openingFolder) =>
        strings.diagnosticsOpenFailed,
      (DiagnosticExportPhase.failed, _) => strings.diagnosticsFailed,
      (DiagnosticExportPhase.saved, _) => strings.diagnosticsSaved,
      _ => strings.diagnosticsCancelled,
    };
    final action = switch ((state.phase, state.failure?.kind)) {
      (DiagnosticExportPhase.saved, _) ||
      (
        DiagnosticExportPhase.failed,
        DiagnosticFailureKind.openingFolder,
      ) => (label: strings.openLocation, run: controller.openFolder),
      (DiagnosticExportPhase.failed, DiagnosticFailureKind.collection) => (
        label: strings.retry,
        run: controller.retry,
      ),
      (DiagnosticExportPhase.failed, _) => (
        label: strings.diagnosticsChooseAgain,
        run: () => controller.export(diagnosticSaveLabels(strings)),
      ),
      _ => null,
    };
    final entry = AmeNotificationEntry(
      id: "diagnostic-${identityHashCode(state)}",
      title: AmeRawMessage(message),
      message: const AmeRawMessage(""),
      detail: nativeCode == null ? null : AmeRawMessage(nativeCode),
      technicalCode: nativeCode,
      severity: failed
          ? AmeNotificationSeverity.error
          : AmeNotificationSeverity.info,
      occurredAt: DateTime.now(),
      isUnread: false,
      isActive: true,
      isPersistent: failed,
      actionId: action == null ? null : "diagnostic-action",
      actionLabel: action == null ? null : AmeRawMessage(action.label),
    );
    return AmeNotificationSurface(
      notification: entry,
      onDismiss: (_) => controller.dismiss(),
      onAction: (_) => unawaited(action?.run()),
    );
  }
}
