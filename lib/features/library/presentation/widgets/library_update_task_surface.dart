import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_theme.dart";
import "../../application/library_update_controller.dart";
import "library_source_navigation_tile.dart";
import "library_task_live_region.dart";

class LibraryUpdateTaskSurface extends StatelessWidget {
  const LibraryUpdateTaskSurface({
    required this.state,
    required this.onCancel,
    required this.onRetry,
    required this.onDismiss,
    required this.onDismissTerminalTasks,
    super.key,
  });

  final LibraryUpdateState state;
  final ValueChanged<String> onCancel;
  final ValueChanged<String> onRetry;
  final ValueChanged<String> onDismiss;
  final VoidCallback onDismissTerminalTasks;

  @override
  Widget build(BuildContext context) {
    final tasks = state.tasks;
    final activeTasks = tasks.where((task) => task.isActive).toList();
    final hasTerminalTasks = tasks.any((task) => !task.isActive);
    final title = activeTasks.length == 1
        ? "正在更新图库“${librarySourceName(activeTasks.single.root.displayPath)}”…"
        : activeTasks.isNotEmpty
        ? "正在更新 ${activeTasks.length} 个图库文件夹…"
        : "图库更新已结束";
    final announcementScope = tasks
        .map((task) => "${task.root.id}:${task.phase.name}")
        .join("|");
    final announcement = [
      title,
      for (final task in tasks)
        "${librarySourceName(task.root.displayPath)}，${_progressSemanticsValue(task)}",
    ].join("。 ");
    final surface = Material(
      key: const Key("library-update-task-surface"),
      elevation: ameNotificationElevation,
      color: Theme.of(context).colorScheme.surfaceContainerHigh,
      borderRadius: BorderRadius.circular(ameNotificationRadius),
      child: ConstrainedBox(
        constraints: const BoxConstraints(
          minWidth: ameNotificationWidth,
          maxWidth: ameNotificationWidth,
          maxHeight: 360,
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 14, 12, 12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  const Icon(Symbols.sync_rounded, size: 20),
                  const SizedBox(width: 12),
                  Expanded(child: Text(title)),
                  if (hasTerminalTasks)
                    TextButton(
                      key: const Key("library-update-dismiss-terminal"),
                      onPressed: onDismissTerminalTasks,
                      child: const Text("清除结果"),
                    ),
                ],
              ),
              const SizedBox(height: 8),
              Flexible(
                child: ListView.separated(
                  shrinkWrap: true,
                  itemCount: tasks.length,
                  separatorBuilder: (_, _) => const Divider(height: 16),
                  itemBuilder: (context, index) {
                    final task = tasks[index];
                    return _LibraryRootUpdateRow(
                      key: ValueKey("library-update-task-${task.root.id}"),
                      task: task,
                      onCancel: () => onCancel(task.root.id),
                      onRetry: () => onRetry(task.root.id),
                      onDismiss: () => onDismiss(task.root.id),
                    );
                  },
                ),
              ),
            ],
          ),
        ),
      ),
    );
    return LibraryTaskLiveRegion(
      message: announcement,
      announcementScope: announcementScope,
      child: surface,
    );
  }
}

class _LibraryRootUpdateRow extends StatelessWidget {
  const _LibraryRootUpdateRow({
    required this.task,
    required this.onCancel,
    required this.onRetry,
    required this.onDismiss,
    super.key,
  });

  final LibraryRootUpdateTask task;
  final VoidCallback onCancel;
  final VoidCallback onRetry;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final rootName = librarySourceName(task.root.displayPath);
    final phaseLabel = _phaseLabel(task.phase);
    final detail =
        task.errorMessage ??
        switch (task.phase) {
          LibraryRootUpdatePhase.queued => "等待可用的更新槽位",
          LibraryRootUpdatePhase.finalizing =>
            "正在核对 ${task.validatedItems} / ${task.validationItemCount} 张图片 · "
                "已检查 ${task.visitedEntries} 个文件",
          LibraryRootUpdatePhase.refreshing => "图库已更新，正在刷新当前视图",
          LibraryRootUpdatePhase.completed =>
            "已检查 ${task.visitedEntries} 个文件 · 图库包含 ${task.acceptedItems} 张图片",
          _ => "已检查 ${task.visitedEntries} 个文件 · 已找到 ${task.acceptedItems} 张图片",
        };
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Expanded(
              child: Text(
                rootName,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
            ),
            Text(
              phaseLabel,
              style: Theme.of(context).textTheme.labelMedium?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(width: 4),
            if (task.canCancel)
              Semantics(
                container: true,
                label: "取消更新 $rootName",
                button: true,
                onTap: onCancel,
                excludeSemantics: true,
                child: TextButton(onPressed: onCancel, child: const Text("取消")),
              )
            else if (task.canRetry)
              Semantics(
                container: true,
                label: "重试更新 $rootName",
                button: true,
                onTap: onRetry,
                excludeSemantics: true,
                child: TextButton(onPressed: onRetry, child: const Text("重试")),
              )
            else if (!task.isActive)
              Semantics(
                container: true,
                label: "清除 $rootName 更新结果",
                button: true,
                onTap: onDismiss,
                excludeSemantics: true,
                child: IconButton(
                  onPressed: onDismiss,
                  icon: const Icon(Symbols.close_rounded),
                ),
              ),
          ],
        ),
        Text(
          detail,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
        if (task.isActive) ...[
          const SizedBox(height: 8),
          Semantics(
            container: true,
            label: "$rootName 更新进度",
            value: _progressSemanticsValue(task),
            child: ExcludeSemantics(
              child: LinearProgressIndicator(
                value:
                    task.phase == LibraryRootUpdatePhase.finalizing &&
                        task.validationItemCount > 0
                    ? task.validatedItems / task.validationItemCount
                    : null,
              ),
            ),
          ),
        ],
      ],
    );
  }
}

String _phaseLabel(LibraryRootUpdatePhase phase) => switch (phase) {
  LibraryRootUpdatePhase.queued => "等待更新",
  LibraryRootUpdatePhase.discovering => "正在更新",
  LibraryRootUpdatePhase.finalizing => "正在核对",
  LibraryRootUpdatePhase.refreshing => "正在刷新显示",
  LibraryRootUpdatePhase.cancelling => "正在取消",
  LibraryRootUpdatePhase.completed => "更新完成",
  LibraryRootUpdatePhase.cancelled => "已取消",
  LibraryRootUpdatePhase.stale => "需要重试",
  LibraryRootUpdatePhase.refreshFailed => "刷新显示失败",
  LibraryRootUpdatePhase.failed => "更新失败",
};

String _progressSemanticsValue(LibraryRootUpdateTask task) {
  final phaseLabel = _phaseLabel(task.phase);
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
