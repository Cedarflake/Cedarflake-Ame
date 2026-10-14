import "../application/library_update_task.dart";
import "library_strings.dart";

class LibraryUpdateFeedback {
  const LibraryUpdateFeedback(this.task);

  final LibraryRootUpdateTask task;

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

  String get retryLabel => switch (task.phase) {
    LibraryRootUpdatePhase.refreshFailed => LibraryStrings.reload,
    LibraryRootUpdatePhase.stale => LibraryStrings.updateLibrary,
    _ => "重试",
  };

  String retrySemanticsLabel(String rootName) => switch (task.phase) {
    LibraryRootUpdatePhase.refreshFailed => "重新加载 $rootName",
    LibraryRootUpdatePhase.stale => "更新图库 $rootName",
    _ => "重试更新 $rootName",
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
