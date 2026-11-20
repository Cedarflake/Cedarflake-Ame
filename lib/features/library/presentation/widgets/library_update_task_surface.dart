import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_motion.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../../../app/presentation/ame_feedback_card.dart";
import "../../application/library_update_controller.dart";
import "../library_update_feedback.dart";
import "library_source_navigation_tile.dart";
import "library_task_live_region.dart";

/// 窗口底部更新任务卡：本组件包含卡片总标题和“清除已结束任务”按钮。
/// 各目录的状态、数量说明和恢复按钮文字见 LibraryUpdateFeedback。
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
    final hasTerminalTasks = tasks.any((task) => !task.isActive);
    final title = context.l10n.libraryUpdates;
    final announcementScope = tasks
        .map((task) => "${task.root.id}:${task.phase.name}")
        .join("|");
    final announcement = [
      title,
      for (final task in tasks)
        [
          librarySourceName(task.root.displayPath),
          LibraryUpdateFeedback(task).progressSemanticsValue(context.l10n),
        ].join(context.l10n.clauseSeparator),
    ].join(context.l10n.sentenceSeparator);
    final surface = AmeFeedbackCard.tasks(
      key: const Key("library-update-task-surface"),
      title: title,
      icon: Symbols.sync_rounded,
      actions: [
        if (hasTerminalTasks)
          AmeFeedbackAction(
            key: const Key("library-update-dismiss-terminal"),
            onPressed: onDismissTerminalTasks,
            label: context.l10n.clearFinishedTasks,
          ),
      ],
      items: AmeScrollMotion(
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
    );
    return LibraryTaskLiveRegion(
      message: announcement,
      announcementScope: announcementScope,
      child: surface,
    );
  }
}

// 每个目录任务行：取消按钮是可见文字；含目录名称的 label 用于屏幕阅读器。
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
    final feedback = LibraryUpdateFeedback(task);
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
              feedback.phaseLabel(context.l10n),
              style: Theme.of(context).textTheme.labelMedium?.copyWith(
                color: Theme.of(context).colorScheme.onSurfaceVariant,
              ),
            ),
            const SizedBox(width: 4),
            if (task.canCancel)
              Semantics(
                container: true,
                label: context.l10n.cancelRootUpdate(rootName),
                button: true,
                onTap: onCancel,
                excludeSemantics: true,
                child: TextButton(
                  onPressed: onCancel,
                  child: Text(context.l10n.cancel),
                ),
              )
            else if (task.canRetry)
              Semantics(
                container: true,
                label: feedback.retrySemanticsLabel(context.l10n, rootName),
                button: true,
                onTap: onRetry,
                excludeSemantics: true,
                child: TextButton(
                  onPressed: onRetry,
                  child: Text(feedback.retryLabel(context.l10n)),
                ),
              )
            else if (!task.isActive)
              Semantics(
                container: true,
                label: context.l10n.closeRootUpdate(rootName),
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
        if (feedback.detail(context.l10n) case final detail?)
          Text(
            detail,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
        AmeStateTransition(
          active: task.isActive,
          motion: AmeMotion.fastSpatial,
          builder: (context, animation, child) => SizeTransition(
            sizeFactor: animation,
            alignment: Alignment.bottomCenter,
            child: child,
          ),
          child: Padding(
            padding: const EdgeInsets.only(top: 8),
            child: task.isActive
                ? Semantics(
                    container: true,
                    label: context.l10n.rootUpdateProgress(rootName),
                    value: feedback.progressSemanticsValue(context.l10n),
                    child: ExcludeSemantics(
                      child: AmeLinearProgress(
                        value:
                            task.phase == LibraryRootUpdatePhase.finalizing &&
                                task.validationItemCount > 0
                            ? task.validatedItems / task.validationItemCount
                            : null,
                      ),
                    ),
                  )
                : const SizedBox(height: 4),
          ),
        ),
      ],
    );
  }
}
