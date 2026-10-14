import "../domain/library_state.dart";
import "library_strings.dart";
import "widgets/library_navigation.dart";

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

  String get title {
    switch (state.status) {
      case LibraryStatus.choosingDirectory:
        return "正在选择文件夹…";
      case LibraryStatus.scanning:
        final name = _rootName(state.displayRootPath);
        if (state.scanPhase == LibraryScanPhase.finalizing) {
          return "正在检查“$name”的图片…";
        }
        return isLibraryUpdate ? "正在更新“$name”…" : "正在添加“$name”…";
      case LibraryStatus.pausing:
        return "正在暂停…";
      case LibraryStatus.cancelling || LibraryStatus.discarding:
        return "正在停止…";
      case LibraryStatus.removing:
        final name = _rootName(state.removingRootDisplayPath);
        return state.isRemovalCommitted
            ? LibraryStrings.refreshingAfterRemoval(name)
            : LibraryStrings.removingFromAme(name);
      case LibraryStatus.refreshing:
        return isConfirmingPublication
            ? LibraryStrings.readingScanResult
            : "正在加载图片…";
      case LibraryStatus.cancelled:
        return isLibraryUpdate ? "已取消更新图库" : "已取消添加文件夹";
      case LibraryStatus.paused:
        return isLibraryUpdate ? "已暂停更新图库" : "已暂停添加文件夹";
      case LibraryStatus.stale:
        return "文件已变化，请重试";
      case LibraryStatus.failed:
        if (isConfirmingPublication) {
          return LibraryStrings.scanConfirmationPending;
        }
        if (isRootRemoval) {
          return state.isRemovalCommitted
              ? LibraryStrings.removedFolderRefreshFailed
              : LibraryStrings.removeFolderFailed;
        }
        if (isPublishedReload) {
          return LibraryStrings.galleryDisplayReloadPending;
        }
        return isLibraryUpdate ? "更新图库失败" : "添加文件夹失败";
      case LibraryStatus.completed:
        return isLibraryUpdate ? "更新完成" : "添加完成";
      case LibraryStatus.empty:
        return "";
    }
  }

  String? get detail {
    if (isConfirmingPublication || isPublishedReload) {
      return null;
    }
    if (state.status == LibraryStatus.failed ||
        state.status == LibraryStatus.paused && state.errorMessage != null) {
      return LibraryStrings.scanFailureDetail(state.errorMessage);
    }
    if (state.status == LibraryStatus.cancelling ||
        state.status == LibraryStatus.discarding) {
      return null;
    }
    if (state.status == LibraryStatus.removing) {
      return state.isRemovalCommitted
          ? LibraryStrings.refreshingAfterRemovalDetail
          : LibraryStrings.removingFolderDetail;
    }
    if (state.status == LibraryStatus.completed) {
      return isLibraryUpdate
          ? "共 ${state.stagedAssetCount} 张图片"
          : "已添加 ${state.stagedAssetCount} 张图片";
    }
    if (state.status == LibraryStatus.scanning &&
        state.scanPhase == LibraryScanPhase.finalizing) {
      return "已检查 ${state.validatedAssetCount} / ${state.validationAssetCount} 张图片 · "
          "已检查 ${state.visitedEntries} 个文件";
    }
    return "已检查 ${state.visitedEntries} 个文件 · 已找到 ${state.stagedAssetCount} 张图片";
  }

  String get retryLabel {
    if (isRootRemoval) {
      return state.isRemovalCommitted ? LibraryStrings.reload : "重试移除";
    }
    return switch (state.primaryScanSnapshot.publication) {
      LibraryScanPublication.confirmationPending =>
        LibraryStrings.rereadScanResult,
      LibraryScanPublication.reloadPending => LibraryStrings.reload,
      _ => LibraryStrings.retry,
    };
  }

  static String _rootName(String? path) =>
      path == null ? "图片" : librarySourceName(path);
}
