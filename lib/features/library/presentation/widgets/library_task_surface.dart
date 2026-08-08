import "package:flutter/material.dart";

import "../../domain/library_state.dart";
import "library_navigation.dart";

class LibraryTaskSurface extends StatelessWidget {
  const LibraryTaskSurface({
    required this.state,
    required this.onPause,
    required this.onCancel,
    required this.onResume,
    required this.onRetry,
    super.key,
  });

  final LibraryState state;
  final VoidCallback onPause;
  final VoidCallback onCancel;
  final Future<void> Function() onResume;
  final Future<void> Function() onRetry;

  @override
  Widget build(BuildContext context) {
    final title = switch (state.status) {
      LibraryStatus.choosingDirectory => "正在选择文件夹…",
      LibraryStatus.scanning => "正在添加文件夹“${_rootName(state.displayRootPath)}”…",
      LibraryStatus.pausing => "正在暂停…",
      LibraryStatus.cancelling => "正在取消…",
      LibraryStatus.refreshing => "正在更新图库…",
      LibraryStatus.cancelled => "已取消添加文件夹",
      LibraryStatus.paused => "已暂停添加文件夹",
      LibraryStatus.stale => "源文件发生变化，需要重新更新",
      LibraryStatus.failed => "添加文件夹失败",
      LibraryStatus.empty || LibraryStatus.completed => "",
    };
    final detail =
        state.errorMessage ??
        "已检查 ${state.visitedEntries} 个文件 · 已找到 ${state.stagedAssetCount} 张图片";
    return Material(
      elevation: 3,
      color: Theme.of(context).colorScheme.surfaceContainerHigh,
      borderRadius: BorderRadius.circular(16),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 680),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 14, 12, 12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  const Icon(Icons.info_outline, size: 20),
                  const SizedBox(width: 12),
                  Expanded(child: Text(title)),
                  if (state.status == LibraryStatus.scanning) ...[
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
                  ] else if (state.status == LibraryStatus.paused)
                    TextButton(
                      key: const Key("library-resume-button"),
                      onPressed: onResume,
                      child: const Text("继续"),
                    )
                  else if (state.status == LibraryStatus.failed ||
                      state.status == LibraryStatus.cancelled ||
                      state.status == LibraryStatus.stale)
                    TextButton(
                      key: const Key("library-retry-button"),
                      onPressed: onRetry,
                      child: const Text("重试"),
                    ),
                ],
              ),
              const SizedBox(height: 4),
              Text(
                detail,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
              if (state.isProcessing) ...[
                const SizedBox(height: 10),
                const LinearProgressIndicator(),
              ],
            ],
          ),
        ),
      ),
    );
  }

  static String _rootName(String? path) {
    if (path == null) {
      return "图片";
    }
    return librarySourceName(path);
  }
}
