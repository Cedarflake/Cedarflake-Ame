import "../application/library_file_operations.dart";
import "../domain/library_file_operations.dart";

abstract final class LibraryFileStrings {
  static String rule(LibraryFileOrganizationRule rule) => switch (rule) {
    LibraryFileOrganizationRule.direct => "直接放入目标文件夹",
    LibraryFileOrganizationRule.category => "按分类",
    LibraryFileOrganizationRule.album => "按相册",
    LibraryFileOrganizationRule.date => "按日期",
    LibraryFileOrganizationRule.source => "按来源文件夹",
  };
  static String action(LibraryFileAction action) => switch (action) {
    LibraryFileAction.copy => "复制",
    LibraryFileAction.move => "移动",
    LibraryFileAction.rename => "重命名",
    LibraryFileAction.recycle || LibraryFileAction.deletePermanently => "删除",
  };

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
    return switch (progress.phase) {
      LibraryFileTaskPhase.preparing => "正在准备$verb",
      LibraryFileTaskPhase.running => "正在$verb",
      LibraryFileTaskPhase.refreshing => "正在更新图库",
      LibraryFileTaskPhase.stopping => "正在停止",
      LibraryFileTaskPhase.finished =>
        progress.failed > 0 || progress.skipped > 0
            ? "$verb已结束，部分文件未处理"
            : "$verb已完成",
      LibraryFileTaskPhase.cancelled => "$verb已停止",
      LibraryFileTaskPhase.needsAttention => "文件操作需要处理",
    };
  }

  static String phase(LibraryFileRecordPhase phase) => switch (phase) {
    LibraryFileRecordPhase.prepared ||
    LibraryFileRecordPhase.nameOccupied => "尚未开始",
    LibraryFileRecordPhase.executing ||
    LibraryFileRecordPhase.removingSource => "需要核对结果",
    LibraryFileRecordPhase.copied => "副本已保存，原文件尚未移除",
    LibraryFileRecordPhase.nativeCompleted => "文件已处理，图库尚未更新",
    LibraryFileRecordPhase.published => "已完成",
    LibraryFileRecordPhase.failed => "未完成",
    LibraryFileRecordPhase.cancelled => "已停止",
    LibraryFileRecordPhase.uncertain => "结果待核对",
  };

  static String failure(LibraryFileFailure? failure) => switch (failure?.code) {
    "organization_selection_changed" ||
    "file_operation_selection_changed" ||
    "file_operation_source_changed" => "所选文件已变化，请刷新图库后重新选择。",
    "file_operation_locations_ambiguous" => "所选图片有多个文件位置，请取消多选后，分别在图片右键菜单中操作。",
    "file_operation_same_destination" => "目标位置与原文件相同，请选择其他文件夹或名称。",
    "file_operation_destination_unavailable" => "无法打开目标文件夹，请重新选择。",
    "file_operation_source_unavailable" => "无法读取所选文件，请刷新图库后重新选择。",
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
