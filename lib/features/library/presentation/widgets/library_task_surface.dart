import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_theme.dart";
import "../../domain/library_state.dart";
import "../library_primary_task_feedback.dart";
import "library_task_live_region.dart";

/// 窗口底部主任务卡；标题、说明和恢复按钮文字来自 LibraryPrimaryTaskFeedback。
/// 暂停、继续、取消、关闭的固定按钮文字位于下方对应状态分支。
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
    final feedback = LibraryPrimaryTaskFeedback(state);
    final title = feedback.title;
    final detail = feedback.detail;
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
                        !feedback.isLibraryUpdate)
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
                      child: Text(feedback.retryLabel),
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
                      child: Text(feedback.retryLabel),
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
    if (!feedback.isRootRemoval) {
      return surface;
    }
    return LibraryTaskLiveRegion(
      message: detail == null ? title : "$title。$detail",
      announcementScope:
          "${state.status.name}:${state.isRemovalCommitted}:${state.removingRootId}",
      child: surface,
    );
  }
}
