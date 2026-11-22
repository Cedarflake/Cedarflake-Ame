import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_feedback_card.dart";
import "../../../library/application/library_archive_controller.dart";
import "../../../library/application/library_archive_state.dart";
import "../../../library/application/library_cleanup_controller.dart";
import "../../../library/application/library_cleanup_state.dart";
import "../organization_archive_messages.dart";
import "../organization_record_actions.dart";

Widget? buildOrganizationRecordFeedback(BuildContext context, WidgetRef ref) {
  final archive = ref.watch(libraryArchiveControllerProvider);
  final cleanup = ref.watch(libraryCleanupControllerProvider);
  final archiveController = ref.read(libraryArchiveControllerProvider.notifier);
  final cleanupController = ref.read(libraryCleanupControllerProvider.notifier);
  if (archive is LibraryArchiveWorking) {
    return _RecordFeedback(
      message: archiveProgressMessage(context.l10n, archive),
      detail: archive.cancelFailed ? context.l10n.stopNotConfirmed : null,
      working: true,
      actionLabel: context.l10n.cancel,
      actionKey: const Key("settings-archive-cancel"),
      onAction: archive.stopping ? null : archiveController.cancel,
    );
  }
  if (cleanup case LibraryCleanupWorking(request: LibraryCleanupApply())) {
    return _RecordFeedback(
      message: cleanup.stopping
          ? context.l10n.stopping
          : context.l10n.clearingRecords,
      detail: cleanup.cancelFailed ? context.l10n.stopNotConfirmed : null,
      working: true,
      actionLabel: context.l10n.cancel,
      actionKey: const Key("settings-cleanup-cancel"),
      onAction: cleanup.stopping ? null : cleanupController.cancel,
    );
  }
  if (archive is LibraryArchiveFailed) {
    return _RecordFeedback(
      message: archiveFailureMessage(context.l10n, archive.failure),
      actionLabel: archive.failure.requiresInspection
          ? context.l10n.checkAgain
          : context.l10n.retry,
      actionKey: const Key("settings-archive-retry"),
      onAction: () => unawaited(retryOrganizationArchive(context)),
      onDismiss: archiveController.dismiss,
    );
  }
  if (cleanup case LibraryCleanupFailed(request: LibraryCleanupApply())) {
    return _RecordFeedback(
      message: cleanup.failure.requiresInspection
          ? context.l10n.recordsChanged
          : context.l10n.cleanupResultUnknown,
      actionLabel: cleanup.failure.requiresInspection
          ? context.l10n.checkAgain
          : context.l10n.retry,
      actionKey: const Key("settings-cleanup-retry"),
      onAction: () => unawaited(
        cleanup.failure.requiresInspection
            ? inspectOrganizationCleanup(context)
            : cleanupController.retry(),
      ),
      onDismiss: cleanupController.dismiss,
    );
  }
  return null;
}

class _RecordFeedback extends StatelessWidget {
  const _RecordFeedback({
    required this.message,
    required this.actionLabel,
    required this.actionKey,
    required this.onAction,
    this.detail,
    this.working = false,
    this.onDismiss,
  });

  final String message;
  final String? detail;
  final bool working;
  final String actionLabel;
  final Key actionKey;
  final VoidCallback? onAction;
  final VoidCallback? onDismiss;

  @override
  Widget build(BuildContext context) => Semantics(
    liveRegion: true,
    child: AmeFeedbackCard(
      key: const Key("organization-record-feedback"),
      title: message,
      message: detail,
      severity: working
          ? AmeNotificationSeverity.info
          : AmeNotificationSeverity.error,
      actions: [
        AmeFeedbackAction(
          key: actionKey,
          onPressed: onAction,
          label: actionLabel,
        ),
        if (onDismiss != null)
          AmeFeedbackAction(onPressed: onDismiss, label: context.l10n.close),
      ],
      progress: working ? const AmeLinearProgress() : null,
    ),
  );
}
