import "../../../app/presentation/ame_localizations.dart";
import "../domain/library_state.dart";
import "library_strings.dart";
import "widgets/library_navigation.dart";

/// 窗口底部主任务卡：添加目录、保留的扫描任务及移除目录的反馈。
/// 暂停、继续、取消、关闭按钮见 widgets/library_task_surface.dart。
class LibraryPrimaryTaskFeedback {
  const LibraryPrimaryTaskFeedback(this.state);

  final LibraryState state;

  bool get isLibraryUpdate => state.taskKind == LibraryTaskKind.update;
  bool get isRootRemoval => state.taskKind == LibraryTaskKind.remove;
  bool get isConfirmingPublication =>
      state.primaryScanSnapshot.publication ==
      LibraryScanPublication.confirmationPending;
  bool get isPublishedReload =>
      state.primaryScanSnapshot.publication ==
      LibraryScanPublication.reloadPending;
  bool get shouldChooseAnotherFolder =>
      state.status == LibraryStatus.failed &&
      state.taskKind == LibraryTaskKind.import &&
      !state.primaryScanSnapshot.hasUnresolvedPublication &&
      (state.errorMessage?.startsWith("source_root_overlaps_storage:") ??
          false);

  // 卡片首行｜按任务阶段选择标题；添加和更新共用此处的状态分支。
  String title(AppLocalizations strings) {
    switch (state.status) {
      case LibraryStatus.choosingDirectory:
        return strings.selectingFolder;
      case LibraryStatus.scanning:
        final name = _rootName(strings, state.displayRootPath);
        if (state.scanPhase == LibraryScanPhase.finalizing) {
          return strings.checkingFolderPhotos(name);
        }
        return isLibraryUpdate
            ? strings.updatingFolder(name)
            : strings.addingFolder(name);
      case LibraryStatus.pausing:
        return strings.pausingEllipsis;
      case LibraryStatus.cancelling || LibraryStatus.discarding:
        return strings.stoppingEllipsis;
      case LibraryStatus.removing:
        final name = _rootName(strings, state.removingRootDisplayPath);
        return state.isRemovalCommitted
            ? strings.refreshingAfterRemoval(name)
            : strings.removingFromAme(name);
      case LibraryStatus.refreshing:
        return isConfirmingPublication
            ? strings.readingScanResult
            : strings.loadingPhotos;
      case LibraryStatus.cancelled:
        return isLibraryUpdate
            ? strings.updateLibraryCancelled
            : strings.addFolderCancelled;
      case LibraryStatus.paused:
        return isLibraryUpdate
            ? strings.updateLibraryPaused
            : strings.addFolderPaused;
      case LibraryStatus.stale:
        return strings.filesChangedRetry;
      case LibraryStatus.failed:
        if (isConfirmingPublication) {
          return strings.scanConfirmationPending;
        }
        if (isRootRemoval) {
          return state.isRemovalCommitted
              ? strings.removedFolderRefreshFailed
              : strings.removeFolderFailed;
        }
        if (isPublishedReload) {
          return strings.galleryDisplayReloadPending;
        }
        return isLibraryUpdate
            ? strings.updateLibraryFailed
            : strings.addFolderFailed;
      case LibraryStatus.completed:
        return isLibraryUpdate ? strings.updateComplete : strings.addComplete;
      case LibraryStatus.empty:
        return "";
    }
  }

  // 标题下方｜检查数量、完成数量、移除说明或已识别的错误原因。
  String? detail(AppLocalizations strings) {
    if (isConfirmingPublication || isPublishedReload) {
      return null;
    }
    if (state.status == LibraryStatus.failed ||
        state.status == LibraryStatus.paused && state.errorMessage != null) {
      return LibraryStrings.scanFailureDetail(strings, state.errorMessage);
    }
    if (state.status == LibraryStatus.cancelling ||
        state.status == LibraryStatus.discarding) {
      return null;
    }
    if (state.status == LibraryStatus.removing) {
      return state.isRemovalCommitted
          ? strings.refreshingAfterRemovalDetail
          : strings.removingFolderDetail;
    }
    if (state.status == LibraryStatus.completed) {
      return isLibraryUpdate
          ? strings.totalPhotos(state.stagedAssetCount)
          : strings.photosAdded(state.stagedAssetCount);
    }
    if (state.status == LibraryStatus.scanning &&
        state.scanPhase == LibraryScanPhase.finalizing) {
      return strings.photosValidated(
        state.validatedAssetCount,
        state.validationAssetCount,
        state.visitedEntries,
      );
    }
    return strings.filesCheckedPhotosFound(
      state.visitedEntries,
      state.stagedAssetCount,
    );
  }

  // 卡片右侧恢复按钮｜失败后按实际动作显示重新读取、重新加载或重试。
  String retryLabel(AppLocalizations strings) {
    if (shouldChooseAnotherFolder) {
      return strings.chooseFolderAgain;
    }
    if (isRootRemoval) {
      return state.isRemovalCommitted
          ? strings.retryLoading
          : strings.retryRemoval;
    }
    return switch (state.primaryScanSnapshot.publication) {
      LibraryScanPublication.confirmationPending => strings.rereadScanResult,
      LibraryScanPublication.reloadPending => strings.retryLoading,
      _ => strings.retry,
    };
  }

  static String _rootName(AppLocalizations strings, String? path) =>
      path == null ? strings.photos : librarySourceName(path);
}
