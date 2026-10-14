import "../application/library_update_task.dart";
import "library_strings.dart";

class LibraryUpdateFeedback {
  const LibraryUpdateFeedback(this.task);

  final LibraryRootUpdateTask task;

  String get phaseLabel => switch (task.phase) {
    LibraryRootUpdatePhase.queued => "等待更新",
    LibraryRootUpdatePhase.discovering => "正在更新",
    LibraryRootUpdatePhase.finalizing => "正在核对",
    LibraryRootUpdatePhase.refreshing => "正在刷新",
    LibraryRootUpdatePhase.cancelling => "正在取消",
    LibraryRootUpdatePhase.completed => "更新完成",
    LibraryRootUpdatePhase.cancelled => "已取消",
    LibraryRootUpdatePhase.stale => "需要重新更新",
    LibraryRootUpdatePhase.refreshFailed => "页面刷新失败",
    LibraryRootUpdatePhase.failed => "更新失败",
  };

  String? get detail {
    return switch (task.phase) {
      LibraryRootUpdatePhase.queued => "正在排队，你可以继续浏览。",
      LibraryRootUpdatePhase.finalizing =>
        "正在核对 ${task.validatedItems} / ${task.validationItemCount} 张图片 · "
            "已检查 ${task.visitedEntries} 个文件",
      LibraryRootUpdatePhase.refreshing => "图库已更新，正在加载最新内容。",
      LibraryRootUpdatePhase.cancelling => "正在停止这次更新，已有图片仍可浏览。",
      LibraryRootUpdatePhase.cancelled => "这次更新已取消，已有图片仍可浏览。",
      LibraryRootUpdatePhase.completed =>
        "已检查 ${task.visitedEntries} 个文件 · 图库包含 ${task.acceptedItems} 张图片",
      LibraryRootUpdatePhase.stale => "更新期间文件发生了变化。",
      LibraryRootUpdatePhase.refreshFailed => "图库更新已保存。",
      LibraryRootUpdatePhase.failed => LibraryStrings.scanFailureDetail(
        task.errorMessage,
      ),
      LibraryRootUpdatePhase.discovering =>
        "已检查 ${task.visitedEntries} 个文件 · 已找到 ${task.acceptedItems} 张图片",
    };
  }

  String get retryLabel => switch (task.phase) {
    LibraryRootUpdatePhase.refreshFailed => LibraryStrings.reload,
    LibraryRootUpdatePhase.stale => "重新更新",
    _ => "重试",
  };

  String retrySemanticsLabel(String rootName) => switch (task.phase) {
    LibraryRootUpdatePhase.refreshFailed => "重新加载 $rootName",
    LibraryRootUpdatePhase.stale => "重新更新 $rootName",
    _ => "重试更新 $rootName",
  };

  String activeTitle(String rootName) => switch (task.phase) {
    LibraryRootUpdatePhase.queued => "“$rootName”正在排队",
    LibraryRootUpdatePhase.cancelling => "正在取消“$rootName”的更新…",
    LibraryRootUpdatePhase.refreshing => "正在刷新“$rootName”…",
    LibraryRootUpdatePhase.finalizing => "正在核对“$rootName”…",
    _ => "正在更新图库“$rootName”…",
  };

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
