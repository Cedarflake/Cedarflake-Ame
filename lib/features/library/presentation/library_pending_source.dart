import "../domain/library_state.dart";
import "library_strings.dart";

/// 左侧栏中新添加目录的临时状态，以及添加目录入口不可用时的悬停提示。
class LibraryPendingSource {
  const LibraryPendingSource._(this.path, this.status, this.publication);

  final String path;
  final LibraryStatus status;
  final LibraryScanPublication publication;

  // 侧栏添加目录按钮｜任务暂停、停止中或扫描结果尚未恢复时，说明暂不能添加的原因。
  static String? addFolderDisabledReason(LibraryPrimaryScanSnapshot scan) {
    if (scan.status == LibraryStatus.discarding) {
      return "正在停止当前任务";
    }
    if (scan.hasRetainedScan) {
      return "请先继续或取消已暂停的任务";
    }
    if (!scan.hasUnresolvedPublication) return null;
    if (scan.status == LibraryStatus.refreshing) {
      return scan.publication == LibraryScanPublication.confirmationPending
          ? LibraryStrings.readingScanResult
          : "正在加载图库…";
    }
    return scan.publication == LibraryScanPublication.confirmationPending
        ? LibraryStrings.addFolderAwaitingScanResult
        : LibraryStrings.addFolderAwaitingGalleryReload;
  }

  static LibraryPendingSource? fromState(LibraryState state) {
    final scan = state.primaryScanSnapshot;
    final path = scan.displayRootPath;
    if (path == null || state.roots.any((root) => root.displayPath == path)) {
      return null;
    }
    return LibraryPendingSource._(path, scan.status, scan.publication);
  }

  bool get isProcessing => switch (status) {
    LibraryStatus.scanning ||
    LibraryStatus.pausing ||
    LibraryStatus.cancelling ||
    LibraryStatus.discarding ||
    LibraryStatus.refreshing => true,
    _ => false,
  };

  // 侧栏新目录名称下方｜尚未进入正式目录列表时的添加和恢复状态。
  String get statusLabel {
    if (publication == LibraryScanPublication.confirmationPending) {
      if (isProcessing) {
        return LibraryStrings.sourceReadingScanResult;
      }
      return LibraryStrings.sourceScanResultReadFailed;
    }
    if (publication == LibraryScanPublication.reloadPending && !isProcessing) {
      return LibraryStrings.sourceDisplayReloadPending;
    }
    return switch (status) {
      LibraryStatus.scanning => LibraryStrings.addingSource,
      LibraryStatus.pausing => LibraryStrings.pausingSource,
      LibraryStatus.cancelling ||
      LibraryStatus.discarding => LibraryStrings.cancellingSource,
      LibraryStatus.refreshing => LibraryStrings.refreshingSourceDisplay,
      LibraryStatus.completed => LibraryStrings.sourceAdded,
      _ => LibraryStrings.firstImportIncomplete,
    };
  }
}
