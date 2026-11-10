import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

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
    final title = LibraryFileStrings.activity(activity);
    final canStop =
        !activity.stopping && progress.phase != LibraryFileTaskPhase.refreshing;
    if (viewer) {
      final action = activity.busy ? commands.cancel : commands.dismiss;
      var message = title;
      if (progress.failure != null) {
        message = LibraryFileStrings.failure(
          progress.failure,
          action: activity.action,
        );
      } else if (activity.purpose != LibraryFileActivityPurpose.report) {
        message =
            "$title · ${progress.processed + progress.skipped}/${progress.total}";
      }
      return LibraryViewerNotice(
        icon: Symbols.folder_rounded,
        message: message,
        actionLabel: activity.busy ? "停止" : "知道了",
        onAction: activity.busy && !canStop ? null : action,
      );
    }
    final message = _message(progress);
    final deletionResult =
        progress.isFinished &&
        progress.failure == null &&
        (activity.action == LibraryFileAction.recycle ||
            activity.action == LibraryFileAction.deletePermanently);
    return AmeFeedbackCard(
      title: deletionResult ? message : title,
      severity:
          progress.failure != null ||
              progress.failed > 0 ||
              progress.skipped > 0
          ? AmeNotificationSeverity.warning
          : AmeNotificationSeverity.info,
      message: deletionResult ? null : message,
      detail: progress.path?.split(RegExp(r"[\\/]")).last,
      progress: activity.busy
          ? LinearProgressIndicator(
              value:
                  activity.purpose != LibraryFileActivityPurpose.report &&
                      progress.total > 0
                  ? (progress.processed + progress.skipped) / progress.total
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
    if (activity.purpose == LibraryFileActivityPurpose.report) {
      if (progress.failure != null) {
        return LibraryFileStrings.failure(progress.failure);
      }
      if (progress.export?.reportState == LibraryFileReportState.notNeeded) {
        return "没有特殊结果，无需生成清单。";
      }
      final path = progress.export?.reportPath;
      return path == null ? "正在读取操作记录" : displayLibraryPath(path);
    }
    if (progress.failure != null) {
      return LibraryFileStrings.failure(
        progress.failure,
        action: activity.action,
      );
    }
    if (progress.phase == LibraryFileTaskPhase.preparing) {
      return "已核对 ${progress.processed} / ${progress.total} 个文件";
    }
    if (progress.isFinished) {
      final skipped = progress.skipped > 0 ? "，已跳过 ${progress.skipped} 个" : "";
      final report = progress.export?.reportPath != null
          ? "\n结果清单已保存到目标文件夹。"
          : "";
      if (activity.action == LibraryFileAction.recycle ||
          activity.action == LibraryFileAction.deletePermanently) {
        final unfinished =
            progress.failed + progress.cancelled + progress.skipped;
        return unfinished == 0
            ? "已删除 ${progress.published} 个文件。"
            : "已删除 ${progress.published} 个，未完成 $unfinished 个。可查看记录了解原因。";
      }
      return "已完成 ${progress.published} 个，未完成 ${progress.failed} 个，已停止 ${progress.cancelled} 个$skipped$report";
    }
    return "已处理 ${progress.processed} / ${progress.total} 个文件";
  }
}
