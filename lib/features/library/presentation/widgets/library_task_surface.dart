import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_theme.dart";
import "../../domain/library_state.dart";
import "../library_strings.dart";
import "library_navigation.dart";
import "library_task_live_region.dart";

class LibraryTaskSurface extends StatelessWidget {
  const LibraryTaskSurface({
    required this.state,
    required this.onPause,
    required this.onCancel,
    required this.onResume,
    required this.onRetry,
    required this.onDismiss,
    super.key,
  });

  final LibraryState state;
  final VoidCallback onPause;
  final VoidCallback onCancel;
  final Future<void> Function() onResume;
  final Future<void> Function() onRetry;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final isLibraryUpdate = state.taskKind == LibraryTaskKind.update;
    final isRootRemoval = state.taskKind == LibraryTaskKind.remove;
    final isConfirmingPublication =
        state.primaryScanSnapshot.publication ==
        LibraryScanPublication.confirmationPending;
    final isPublishedReload =
        state.primaryScanSnapshot.publication ==
        LibraryScanPublication.reloadPending;
    final title = switch (state.status) {
      LibraryStatus.choosingDirectory => "正在选择文件夹…",
      LibraryStatus.scanning =>
        isLibraryUpdate && state.scanPhase == LibraryScanPhase.finalizing
            ? "正在检查“${_rootName(state.displayRootPath)}”的图片…"
            : isLibraryUpdate
            ? "正在更新“${_rootName(state.displayRootPath)}”…"
            : state.scanPhase == LibraryScanPhase.finalizing
            ? "正在检查“${_rootName(state.displayRootPath)}”的图片…"
            : "正在添加“${_rootName(state.displayRootPath)}”…",
      LibraryStatus.pausing => "正在暂停…",
      LibraryStatus.cancelling || LibraryStatus.discarding => "正在停止…",
      LibraryStatus.removing =>
        state.isRemovalCommitted
            ? LibraryStrings.refreshingAfterRemoval(
                _rootName(state.removingRootDisplayPath),
              )
            : LibraryStrings.removingFromAme(
                _rootName(state.removingRootDisplayPath),
              ),
      LibraryStatus.refreshing when isConfirmingPublication =>
        LibraryStrings.readingScanResult,
      LibraryStatus.refreshing => "正在加载图片…",
      LibraryStatus.cancelled => isLibraryUpdate ? "已取消更新图库" : "已取消添加文件夹",
      LibraryStatus.paused => isLibraryUpdate ? "已暂停更新图库" : "已暂停添加文件夹",
      LibraryStatus.stale => "文件已变化，请重试",
      LibraryStatus.failed when isConfirmingPublication =>
        LibraryStrings.scanConfirmationPending,
      LibraryStatus.failed =>
        isRootRemoval
            ? state.isRemovalCommitted
                  ? LibraryStrings.removedFolderRefreshFailed
                  : LibraryStrings.removeFolderFailed
            : isPublishedReload
            ? LibraryStrings.galleryDisplayReloadPending
            : isLibraryUpdate
            ? "更新图库失败"
            : "添加文件夹失败",
      LibraryStatus.completed => isLibraryUpdate ? "更新完成" : "添加完成",
      LibraryStatus.empty => "",
    };
    final completedDetail = isLibraryUpdate
        ? "已检查 ${state.visitedEntries} 个文件 · 图库包含 ${state.stagedAssetCount} 张图片"
        : "已检查 ${state.visitedEntries} 个文件 · 已添加 ${state.stagedAssetCount} 张图片";
    final isFinalizing =
        state.status == LibraryStatus.scanning &&
        state.scanPhase == LibraryScanPhase.finalizing;
    final scanningDetail = isFinalizing
        ? "已检查 ${state.validatedAssetCount} / ${state.validationAssetCount} 张图片 · "
              "已检查 ${state.visitedEntries} 个文件"
        : "已检查 ${state.visitedEntries} 个文件 · 已找到 ${state.stagedAssetCount} 张图片";
    final String? detail;
    if (isConfirmingPublication || isPublishedReload) {
      detail = null;
    } else if (state.status == LibraryStatus.failed) {
      detail = LibraryStrings.scanFailureDetail(state.errorMessage);
    } else if (state.status == LibraryStatus.cancelling ||
        state.status == LibraryStatus.discarding) {
      detail = null;
    } else if (state.status == LibraryStatus.removing) {
      detail = state.isRemovalCommitted
          ? LibraryStrings.refreshingAfterRemovalDetail
          : LibraryStrings.removingFolderDetail;
    } else {
      detail = state.status == LibraryStatus.completed
          ? completedDetail
          : scanningDetail;
    }
    final surface = Material(
      key: const Key("library-task-surface"),
      elevation: ameNotificationElevation,
      color: Theme.of(context).colorScheme.surfaceContainerHigh,
      borderRadius: BorderRadius.circular(ameNotificationRadius),
      child: ConstrainedBox(
        constraints: const BoxConstraints.tightFor(width: ameNotificationWidth),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 14, 12, 12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  const Icon(Symbols.info_rounded, size: 20),
                  const SizedBox(width: 12),
                  Expanded(child: Text(title)),
                  if (state.status == LibraryStatus.scanning ||
                      state.status == LibraryStatus.pausing) ...[
                    if (state.status == LibraryStatus.scanning &&
                        !isLibraryUpdate)
                      TextButton(
                        key: const Key("library-pause-button"),
                        onPressed: onPause,
                        child: const Text("暂停"),
                      ),
                    TextButton(
                      key: const Key("library-cancel-button"),
                      onPressed: onCancel,
                      child: const Text("取消"),
                    ),
                  ] else if (state.status == LibraryStatus.paused) ...[
                    TextButton(
                      key: const Key("library-resume-button"),
                      onPressed: onResume,
                      child: const Text("继续"),
                    ),
                    TextButton(
                      key: const Key("library-cancel-button"),
                      onPressed: onCancel,
                      child: const Text("取消"),
                    ),
                  ] else if (state.status == LibraryStatus.discarding) ...[
                    const TextButton(onPressed: null, child: Text("继续")),
                    const TextButton(onPressed: null, child: Text("取消")),
                  ] else if (state.status == LibraryStatus.failed ||
                      state.status == LibraryStatus.cancelled) ...[
                    TextButton(
                      key: const Key("library-retry-button"),
                      onPressed: onRetry,
                      child: Text(_retryLabel),
                    ),
                    if (!state.isCommittedRemovalReloadPending &&
                        !state.primaryScanSnapshot.hasUnresolvedPublication)
                      TextButton(
                        key: const Key("library-task-dismiss-button"),
                        onPressed: onDismiss,
                        child: const Text("关闭"),
                      ),
                  ] else if (state.status == LibraryStatus.stale)
                    TextButton(
                      key: const Key("library-retry-button"),
                      onPressed: onRetry,
                      child: Text(_retryLabel),
                    )
                  else if (state.status == LibraryStatus.completed)
                    TextButton(
                      key: const Key("library-task-dismiss-button"),
                      onPressed: onDismiss,
                      child: const Text("关闭"),
                    ),
                ],
              ),
              if (detail != null) ...[
                const SizedBox(height: 4),
                Text(
                  detail,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
              if (state.isTaskProcessing) ...[
                const SizedBox(height: 10),
                LinearProgressIndicator(
                  value:
                      state.status == LibraryStatus.scanning &&
                          state.scanPhase == LibraryScanPhase.finalizing &&
                          state.validationAssetCount > 0
                      ? state.validatedAssetCount / state.validationAssetCount
                      : null,
                ),
              ],
            ],
          ),
        ),
      ),
    );
    if (!isRootRemoval) {
      return surface;
    }
    return LibraryTaskLiveRegion(
      message: detail == null ? title : "$title。$detail",
      announcementScope:
          "${state.status.name}:${state.isRemovalCommitted}:${state.removingRootId}",
      child: surface,
    );
  }

  String get _retryLabel {
    if (state.taskKind == LibraryTaskKind.remove) {
      return state.isRemovalCommitted ? LibraryStrings.reload : "重试移除";
    }
    return switch (state.primaryScanSnapshot.publication) {
      LibraryScanPublication.confirmationPending =>
        LibraryStrings.rereadScanResult,
      LibraryScanPublication.reloadPending => LibraryStrings.reload,
      _ => LibraryStrings.retry,
    };
  }

  static String _rootName(String? path) {
    if (path == null) {
      return "图片";
    }
    return librarySourceName(path);
  }
}
