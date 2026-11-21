import "../../../app/presentation/ame_localizations.dart";
import "../application/library_update_task.dart";
import "library_strings.dart";

/// 窗口底部“图库更新”卡片中，每个目录任务行的文案。
/// 卡片总标题、取消和关闭控件见 widgets/library_update_task_surface.dart。
class LibraryUpdateFeedback {
  const LibraryUpdateFeedback(this.task);

  final LibraryRootUpdateTask task;

  // 目录名称右侧｜等待、进行、停止、完成和失败的状态文字。
  String phaseLabel(AppLocalizations strings) => switch (task.phase) {
    LibraryRootUpdatePhase.queued => strings.waitingForUpdate,
    LibraryRootUpdatePhase.discovering => strings.synchronizing,
    LibraryRootUpdatePhase.finalizing => strings.checkingPhotos,
    LibraryRootUpdatePhase.refreshing => strings.loadingPhotosShort,
    LibraryRootUpdatePhase.cancelling => strings.stopping,
    LibraryRootUpdatePhase.completed => strings.updateComplete,
    LibraryRootUpdatePhase.cancelled => strings.cancelled,
    LibraryRootUpdatePhase.stale => strings.filesChanged,
    LibraryRootUpdatePhase.refreshFailed => strings.galleryDisplayReloadPending,
    LibraryRootUpdatePhase.failed => strings.updateFailed,
  };

  // 目录任务行下方｜检查数量、完成数量或失败说明。
  String? detail(AppLocalizations strings) {
    return switch (task.phase) {
      LibraryRootUpdatePhase.queued ||
      LibraryRootUpdatePhase.cancelling ||
      LibraryRootUpdatePhase.cancelled ||
      LibraryRootUpdatePhase.stale => null,
      LibraryRootUpdatePhase.finalizing => strings.photosValidated(
        task.validatedItems,
        task.validationItemCount,
        task.visitedEntries,
      ),
      LibraryRootUpdatePhase.refreshing => null,
      LibraryRootUpdatePhase.completed => strings.totalPhotos(
        task.acceptedItems,
      ),
      LibraryRootUpdatePhase.refreshFailed =>
        LibraryStrings.scanFailureDetail(strings, task.errorMessage) ??
            strings.scanCompleted,
      LibraryRootUpdatePhase.failed => LibraryStrings.scanFailureDetail(
        strings,
        task.errorMessage,
      ),
      LibraryRootUpdatePhase.discovering => strings.filesCheckedPhotosFound(
        task.visitedEntries,
        task.acceptedItems,
      ),
    };
  }

  // 目录任务行右侧｜失败后可点击的恢复按钮。
  String retryLabel(AppLocalizations strings) => switch (task.phase) {
    LibraryRootUpdatePhase.refreshFailed => strings.retryLoading,
    LibraryRootUpdatePhase.stale => strings.updateLibrary,
    _ => strings.retry,
  };

  // 屏幕阅读器｜恢复按钮的朗读标签，包含目录名称；不作为可见按钮文字。
  String retrySemanticsLabel(AppLocalizations strings, String rootName) =>
      switch (task.phase) {
        LibraryRootUpdatePhase.refreshFailed => strings.reloadRoot(rootName),
        LibraryRootUpdatePhase.stale => strings.updateRoot(rootName),
        _ => strings.retryRoot(rootName),
      };

  // 屏幕阅读器｜进度条和任务状态播报使用的完整进度说明。
  String progressSemanticsValue(AppLocalizations strings) {
    if (task.phase == LibraryRootUpdatePhase.finalizing &&
        task.validationItemCount > 0) {
      final percent = ((task.validatedItems / task.validationItemCount) * 100)
          .round()
          .clamp(0, 100);
      return strings.progressPhotos(
        phaseLabel(strings),
        task.validatedItems,
        task.validationItemCount,
        percent,
      );
    }
    if (task.phase == LibraryRootUpdatePhase.discovering) {
      return strings.progressFiles(
        phaseLabel(strings),
        task.visitedEntries,
        task.acceptedItems,
      );
    }
    return phaseLabel(strings);
  }
}
