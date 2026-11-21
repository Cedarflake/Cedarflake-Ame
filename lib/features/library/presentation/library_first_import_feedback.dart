import "../../../app/presentation/ame_localizations.dart";
import "../domain/library_state.dart";

/// 图库中央的首次添加提示：为目录图标下方的标题和说明选择对应状态。
/// 操作按钮由底部主任务卡提供。
class LibraryFirstImportFeedback {
  const LibraryFirstImportFeedback._(this.title, this.detail);

  factory LibraryFirstImportFeedback.fromState(
    AppLocalizations strings,
    LibraryState state,
  ) {
    final scan = state.primaryScanSnapshot;
    if (scan.taskKind != LibraryTaskKind.import ||
        !_matchesSelectedRoot(state, scan)) {
      return _incomplete(strings);
    }
    return switch (scan.status) {
      LibraryStatus.scanning => LibraryFirstImportFeedback._(
        strings.firstImportInProgress,
        strings.firstImportInProgressDetail,
      ),
      LibraryStatus.pausing => LibraryFirstImportFeedback._(
        strings.pausingSource,
        null,
      ),
      LibraryStatus.paused => LibraryFirstImportFeedback._(
        strings.firstImportPaused,
        strings.firstImportPausedDetail,
      ),
      LibraryStatus.cancelling || LibraryStatus.discarding =>
        LibraryFirstImportFeedback._(strings.cancellingSource, null),
      LibraryStatus.cancelled => LibraryFirstImportFeedback._(
        strings.firstImportCancelled,
        strings.firstImportGalleryDetail,
      ),
      LibraryStatus.failed ||
      LibraryStatus.stale => LibraryFirstImportFeedback._(
        strings.firstImportIncomplete,
        strings.firstImportRetryDetail,
      ),
      _ => _incomplete(strings),
    };
  }

  final String title;
  final String? detail;

  static LibraryFirstImportFeedback _incomplete(AppLocalizations strings) =>
      LibraryFirstImportFeedback._(
        strings.firstImportIncomplete,
        strings.firstImportGalleryDetail,
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
