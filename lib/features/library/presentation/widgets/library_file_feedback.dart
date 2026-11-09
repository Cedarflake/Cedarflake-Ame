import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_feedback_card.dart";
import "../../application/library_file_operations.dart";
import "../../domain/library_file_operations.dart";
import "../library_file_strings.dart";
import "library_file_history.dart";
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
    final title = LibraryFileStrings.activity(activity);
    final canStop =
        !activity.stopping && progress.phase != LibraryFileTaskPhase.refreshing;
    if (viewer) {
      final action = activity.busy ? commands.cancel : commands.dismiss;
      return LibraryViewerNotice(
        icon: Symbols.folder_rounded,
        message: "$title · ${progress.processed}/${progress.total}",
        actionLabel: activity.busy ? "停止" : "知道了",
        onAction: activity.busy && !canStop ? null : action,
      );
    }
    return AmeFeedbackCard(
      title: title,
      severity: progress.failure != null || progress.failed > 0
          ? AmeNotificationSeverity.warning
          : AmeNotificationSeverity.info,
      message: _message(progress),
      detail: progress.path?.split(RegExp(r"[\\/]")).last,
      progress: activity.busy
          ? LinearProgressIndicator(
              value: progress.total > 0
                  ? progress.processed / progress.total
                  : null,
            )
          : null,
      actions: [
        AmeFeedbackAction(
          label: "查看记录",
          onPressed: () => showLibraryFileHistory(context),
        ),
        if (activity.busy)
          AmeFeedbackAction(
            label: "停止",
            onPressed: canStop ? commands.cancel : null,
          )
        else
          AmeFeedbackAction(label: "知道了", onPressed: commands.dismiss),
      ],
    );
  }

  String _message(LibraryFileProgress progress) {
    if (progress.failure != null) {
      return LibraryFileStrings.failure(progress.failure);
    }
    if (progress.phase == LibraryFileTaskPhase.preparing) {
      return "已核对 ${progress.processed} / ${progress.total} 个文件";
    }
    if (progress.isFinished) {
      return "已完成 ${progress.published} 个，未完成 ${progress.failed} 个，已停止 ${progress.cancelled} 个";
    }
    return "已处理 ${progress.processed} / ${progress.total} 个文件";
  }
}
