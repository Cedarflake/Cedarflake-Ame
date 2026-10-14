import "../application/library_update_task.dart";
import "library_strings.dart";

/// 窗口底部“图库更新”卡片中，每个目录任务行的文案。
/// 卡片总标题、取消和关闭控件见 widgets/library_update_task_surface.dart。
class LibraryUpdateFeedback {
  const LibraryUpdateFeedback(this.task);

  final LibraryRootUpdateTask task;

  // 目录名称右侧｜等待、进行、停止、完成和失败的状态文字。
  String get phaseLabel => switch (task.phase) {
    LibraryRootUpdatePhase.queued => "等待更新",
    LibraryRootUpdatePhase.discovering => "正在更新",
    LibraryRootUpdatePhase.finalizing => "正在检查图片",
    LibraryRootUpdatePhase.refreshing => "正在加载图片",
    LibraryRootUpdatePhase.cancelling => "正在停止",
    LibraryRootUpdatePhase.completed => "更新完成",
    LibraryRootUpdatePhase.cancelled => "已取消",
    LibraryRootUpdatePhase.stale => "文件已变化",
    LibraryRootUpdatePhase.refreshFailed => "图片加载失败",
    LibraryRootUpdatePhase.failed => "更新失败",
  };

  // 目录任务行下方｜检查数量、完成数量或失败说明。
  String? get detail {
    return switch (task.phase) {
      LibraryRootUpdatePhase.queued ||
      LibraryRootUpdatePhase.cancelling ||
      LibraryRootUpdatePhase.cancelled ||
      LibraryRootUpdatePhase.stale => null,
      LibraryRootUpdatePhase.finalizing =>
        "已检查 ${task.validatedItems} / ${task.validationItemCount} 张图片 · "
            "已检查 ${task.visitedEntries} 个文件",
      LibraryRootUpdatePhase.refreshing => null,
      LibraryRootUpdatePhase.completed => "共 ${task.acceptedItems} 张图片",
      LibraryRootUpdatePhase.refreshFailed =>
        LibraryStrings.scanFailureDetail(task.errorMessage) ?? "扫描已完成。",
      LibraryRootUpdatePhase.failed => LibraryStrings.scanFailureDetail(
        task.errorMessage,
      ),
      LibraryRootUpdatePhase.discovering =>
        "已检查 ${task.visitedEntries} 个文件 · 已找到 ${task.acceptedItems} 张图片",
    };
  }

  // 目录任务行右侧｜失败后可点击的恢复按钮。
  String get retryLabel => switch (task.phase) {
    LibraryRootUpdatePhase.refreshFailed => LibraryStrings.reload,
    LibraryRootUpdatePhase.stale => LibraryStrings.updateLibrary,
    _ => "重试",
  };

  // 屏幕阅读器｜恢复按钮的朗读标签，包含目录名称；不作为可见按钮文字。
  String retrySemanticsLabel(String rootName) => switch (task.phase) {
    LibraryRootUpdatePhase.refreshFailed => "重新加载 $rootName",
    LibraryRootUpdatePhase.stale => "更新图库 $rootName",
    _ => "重试更新 $rootName",
  };

  // 屏幕阅读器｜进度条和任务状态播报使用的完整进度说明。
  String get progressSemanticsValue {
    if (task.phase == LibraryRootUpdatePhase.finalizing &&
        task.validationItemCount > 0) {
      final percent = ((task.validatedItems / task.validationItemCount) * 100)
          .round()
          .clamp(0, 100);
      return "$phaseLabel，${task.validatedItems} / "
          "${task.validationItemCount} 张图片，$percent%";
    }
    if (task.phase == LibraryRootUpdatePhase.discovering) {
      return "$phaseLabel，已检查 ${task.visitedEntries} 个文件，"
          "已找到 ${task.acceptedItems} 张图片";
    }
    return phaseLabel;
  }
}
