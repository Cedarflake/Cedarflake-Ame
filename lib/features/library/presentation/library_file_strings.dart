import "../application/library_file_operations.dart";
import "../domain/library_file_operations.dart";

abstract final class LibraryFileStrings {
  static const changedBeforeDeletion = "文件已更改，未删除。请查看最新图片后重新选择并确认。";
  static String deletionTitle(LibraryFileSelectionSummary summary) {
    if (summary.cleanup != null) {
      return summary.files == 0 ? "没有可删除的图片" : "即将删除 ${summary.files} 张图片";
    }
    final groups = summary.mergedGroups;
    if (groups == null || summary.files == groups) {
      return "删除 ${summary.files} 个文件？";
    }
    if (groups == 1) return "删除这组图片的全部 ${summary.files} 个文件？";
    return "删除 $groups 组图片，共 ${summary.files} 个文件？";
  }

  static String rule(LibraryFileOrganizationRule rule) => switch (rule) {
    LibraryFileOrganizationRule.direct => "直接放入目标文件夹",
    LibraryFileOrganizationRule.category => "按分类",
    LibraryFileOrganizationRule.album => "按相册",
    LibraryFileOrganizationRule.date => "按日期",
    LibraryFileOrganizationRule.source => "按来源文件夹",
    LibraryFileOrganizationRule.albumYear => "按相册与年份",
  };
  static String ruleDescription(LibraryFileOrganizationRule rule) =>
      switch (rule) {
        LibraryFileOrganizationRule.date => "按图库日期建立 YYYY-MM 文件夹；缺少日期的文件会跳过。",
        LibraryFileOrganizationRule.albumYear => "按图库日期放入相册名／年份文件夹，例如：旅行／2024。",
        _ => "无法确定目标的文件会跳过，并在目标文件夹中生成结果清单。",
      };
  static String action(LibraryFileAction action) => switch (action) {
    LibraryFileAction.copy => "复制",
    LibraryFileAction.move => "移动",
    LibraryFileAction.rename => "重命名",
    LibraryFileAction.recycle || LibraryFileAction.deletePermanently => "删除",
  };

  static String result(
    LibraryFileProgress progress,
    LibraryFileAction? action,
  ) {
    final deletion =
        action == LibraryFileAction.recycle ||
        action == LibraryFileAction.deletePermanently;
    final outcomes = [
      if (progress.published > 0)
        "${deletion ? '已删除' : '已完成'} ${progress.published} 个文件",
      if (progress.failed > 0) "未完成 ${progress.failed} 个",
      if (progress.cancelled > 0) "已停止 ${progress.cancelled} 个",
      if (progress.skipped > 0) "已跳过 ${progress.skipped} 个",
    ];
    return outcomes.isEmpty ? "未处理任何文件。" : "${outcomes.join('，')}。";
  }

  static String activity(LibraryFileActivity activity) {
    final progress = activity.progress!;
    if (activity.stopping) return "正在停止";
    if (activity.purpose == LibraryFileActivityPurpose.report) {
      if (progress.phase == LibraryFileTaskPhase.finished) {
        return progress.export?.reportState == LibraryFileReportState.notNeeded
            ? "无需生成结果清单"
            : "结果清单已保存";
      }
      if (progress.phase == LibraryFileTaskPhase.needsAttention) {
        return "结果清单未能生成";
      }
    }
    final verb = switch (activity.purpose) {
      LibraryFileActivityPurpose.action => action(activity.action!),
      LibraryFileActivityPurpose.recovery => "核对文件操作",
      LibraryFileActivityPurpose.report => "生成结果清单",
    };
    final status = switch (progress.phase) {
      LibraryFileTaskPhase.preparing => "正在准备$verb",
      LibraryFileTaskPhase.running ||
      LibraryFileTaskPhase.refreshing => "正在$verb",
      LibraryFileTaskPhase.stopping => "正在停止",
      LibraryFileTaskPhase.finished => "$verb已结束",
      LibraryFileTaskPhase.cancelled => "$verb已停止",
      LibraryFileTaskPhase.needsAttention => "文件操作需要处理",
    };
    if (!progress.isFinished ||
        progress.failure != null ||
        activity.purpose == LibraryFileActivityPurpose.report) {
      return status;
    }
    final summary = result(progress, activity.action);
    return activity.action == LibraryFileAction.recycle ||
            activity.action == LibraryFileAction.deletePermanently
        ? summary
        : "$status，$summary";
  }

  static String phase(LibraryFileRecordPhase phase) => switch (phase) {
    LibraryFileRecordPhase.prepared ||
    LibraryFileRecordPhase.nameOccupied => "尚未开始",
    LibraryFileRecordPhase.executing ||
    LibraryFileRecordPhase.removingSource => "需要核对结果",
    LibraryFileRecordPhase.copied => "移动未完成",
    LibraryFileRecordPhase.nativeCompleted => "文件已处理，图库尚未更新",
    LibraryFileRecordPhase.published => "已完成",
    LibraryFileRecordPhase.failed => "未完成",
    LibraryFileRecordPhase.cancelled => "已停止",
    LibraryFileRecordPhase.uncertain => "结果待核对",
  };

  static String failure(
    LibraryFileFailure? failure, {
    LibraryFileAction? action,
  }) => switch (failure?.code) {
    "organization_evidence_pending" => "重复组尚未核对完成，请稍后重试。",
    "organization_selection_changed" ||
    "file_operation_selection_changed" => "所选图片已变化，请查看当前图库后重新选择。",
    "file_operation_source_changed" =>
      action == LibraryFileAction.recycle ||
              action == LibraryFileAction.deletePermanently
          ? changedBeforeDeletion
          : "所选文件已变化，未执行操作。请查看最新文件后重新选择。",
    "file_operation_source_unverified" =>
      action == LibraryFileAction.recycle ||
              action == LibraryFileAction.deletePermanently
          ? "文件已变化或无法核对，未删除。请查看最新图片后重新选择并确认。"
          : "文件已变化或无法核对，未执行操作。请查看最新文件后重新选择。",
    "file_operation_locations_ambiguous" => "所选图片有多个文件位置，请取消多选后，分别在图片右键菜单中操作。",
    "file_operation_same_destination" => "目标位置与原文件相同，请选择其他文件夹或名称。",
    "file_operation_destination_unavailable" => "无法打开目标文件夹，请重新选择。",
    "file_operation_source_unavailable" => "无法读取所选文件，请刷新图库后重新选择。",
    "file_operation_keep_unavailable" => "要保留的图片已变化或无法读取，尚未删除文件。请重新选择保留图片。",
    "file_operation_name_invalid" => "文件名不可用，请换一个名称。",
    "file_operation_preparation_failed" => "未能准备文件操作，请重新选择后重试。",
    "file_operation_busy" ||
    "file_operation_selection_busy" => "还有文件操作正在处理，请稍后重试。",
    "file_operation_inspection_required" => "请在文件操作记录中核对结果后再继续。",
    "file_operation_selection_closed" => "这次操作已取消。",
    "file_export_report_failed" => "文件操作结果已保留，但结果清单未能写入。可在记录中重新生成清单。",
    _ => "部分文件可能尚未处理，请查看文件操作记录。",
  };
}
