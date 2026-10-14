import "../domain/library_state.dart";
import "library_strings.dart";

/// 图库中央的首次添加提示：为目录图标下方的标题和说明选择对应状态。
/// 具体文字见 LibraryStrings 的“首次添加”分组；操作按钮由底部主任务卡提供。
class LibraryFirstImportFeedback {
  const LibraryFirstImportFeedback._(this.title, this.detail);

  factory LibraryFirstImportFeedback.fromState(LibraryState state) {
    final scan = state.primaryScanSnapshot;
    if (scan.taskKind != LibraryTaskKind.import ||
        !_matchesSelectedRoot(state, scan)) {
      return _incomplete;
    }
    return switch (scan.status) {
      LibraryStatus.scanning => const LibraryFirstImportFeedback._(
        LibraryStrings.firstImportInProgress,
        LibraryStrings.firstImportInProgressDetail,
      ),
      LibraryStatus.pausing => const LibraryFirstImportFeedback._(
        LibraryStrings.pausingSource,
        null,
      ),
      LibraryStatus.paused => const LibraryFirstImportFeedback._(
        LibraryStrings.firstImportPaused,
        LibraryStrings.firstImportPausedDetail,
      ),
      LibraryStatus.cancelling ||
      LibraryStatus.discarding => const LibraryFirstImportFeedback._(
        LibraryStrings.cancellingSource,
        null,
      ),
      LibraryStatus.cancelled => const LibraryFirstImportFeedback._(
        LibraryStrings.firstImportCancelled,
        LibraryStrings.firstImportGalleryDetail,
      ),
      LibraryStatus.failed ||
      LibraryStatus.stale => const LibraryFirstImportFeedback._(
        LibraryStrings.firstImportIncomplete,
        LibraryStrings.firstImportRetryDetail,
      ),
      _ => _incomplete,
    };
  }

  final String title;
  final String? detail;

  static const _incomplete = LibraryFirstImportFeedback._(
    LibraryStrings.firstImportIncomplete,
    LibraryStrings.firstImportGalleryDetail,
  );

  static bool _matchesSelectedRoot(
    LibraryState state,
    LibraryPrimaryScanSnapshot scan,
  ) {
    final selectedRootId = state.query.rootId;
    if (selectedRootId == null) {
      return true;
    }
    return state.roots.any(
      (root) =>
          root.id == selectedRootId &&
          (root.path == scan.rootPath ||
              root.displayPath == scan.displayRootPath),
    );
  }
}
