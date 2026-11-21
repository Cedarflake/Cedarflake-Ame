import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_feedback_card.dart";
import "../../application/library_file_operations.dart";
import "../../domain/library_file_operations.dart";
import "../library_file_strings.dart";
import "library_file_history.dart";
import "library_path_text.dart";
import "library_viewer_notice.dart";

class LibraryFileFeedback extends ConsumerWidget {
  const LibraryFileFeedback({
    required this.activity,
    this.viewer = false,
    super.key,
  });
  final LibraryFileActivity activity;
  final bool viewer;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final progress = activity.progress!;
    final commands = ref.read(libraryFileActionsProvider.notifier);
    final title = LibraryFileStrings.activity(context.l10n, activity);
    final canStop =
        !activity.stopping && progress.phase != LibraryFileTaskPhase.refreshing;
    if (viewer) {
      final action = activity.busy ? commands.cancel : commands.dismiss;
      var message = title;
      if (progress.failure != null) {
        message = LibraryFileStrings.failure(
          context.l10n,
          progress.failure,
          action: activity.action,
        );
      } else if (progress.isFinished &&
          activity.purpose != LibraryFileActivityPurpose.report) {
        message = LibraryFileStrings.result(
          context.l10n,
          progress,
          activity.action,
        );
      } else if (activity.purpose != LibraryFileActivityPurpose.report &&
          progress.phase != LibraryFileTaskPhase.refreshing &&
          progress.processed + progress.skipped < progress.total) {
        message =
            "$title · ${progress.processed + progress.skipped}/${progress.total}";
      }
      return LibraryViewerNotice(
        icon: Symbols.folder_rounded,
        message: message,
        actionLabel: activity.busy
            ? context.l10n.stop
            : context.l10n.acknowledge,
        onAction: activity.busy && !canStop ? null : action,
      );
    }
    final message = _message(context.l10n, progress);
    return AmeFeedbackCard(
      title: title,
      severity:
          progress.failure != null ||
              progress.failed > 0 ||
              progress.skipped > 0
          ? AmeNotificationSeverity.warning
          : AmeNotificationSeverity.info,
      message: message,
      detail: activity.busy
          ? progress.path?.split(RegExp(r"[\\/]")).last
          : null,
      progress: activity.busy
          ? AmeLinearProgress(
              value:
                  activity.purpose != LibraryFileActivityPurpose.report &&
                      progress.total > 0 &&
                      progress.processed + progress.skipped < progress.total
                  ? (progress.processed + progress.skipped) / progress.total
                  : null,
            )
          : null,
      actions: [
        AmeFeedbackAction(
          label: context.l10n.viewHistory,
          onPressed: () => showLibraryFileHistory(context),
        ),
        if (activity.busy)
          AmeFeedbackAction(
            label: context.l10n.stop,
            onPressed: canStop ? commands.cancel : null,
          )
        else
          AmeFeedbackAction(
            label: context.l10n.acknowledge,
            onPressed: commands.dismiss,
          ),
      ],
    );
  }

  String? _message(AppLocalizations strings, LibraryFileProgress progress) {
    if (activity.purpose == LibraryFileActivityPurpose.report) {
      if (progress.failure != null) {
        return LibraryFileStrings.failure(strings, progress.failure);
      }
      if (progress.export?.reportState == LibraryFileReportState.notNeeded) {
        return strings.noExceptionsReport;
      }
      final path = progress.export?.reportPath;
      return path == null
          ? strings.loadingOperationHistory
          : displayLibraryPath(path);
    }
    if (progress.failure != null) {
      return LibraryFileStrings.failure(
        strings,
        progress.failure,
        action: activity.action,
      );
    }
    if (progress.phase == LibraryFileTaskPhase.preparing) {
      return strings.verifiedFiles(progress.processed, progress.total);
    }
    if (progress.isFinished) {
      return progress.export?.reportPath != null
          ? strings.reportSavedDestination
          : null;
    }
    if (progress.phase == LibraryFileTaskPhase.refreshing ||
        progress.total > 0 &&
            progress.processed + progress.skipped >= progress.total) {
      return null;
    }
    return strings.processedFiles(progress.processed, progress.total);
  }
}
