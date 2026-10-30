import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../library/application/library_cleanup_controller.dart";
import "../../../library/application/library_cleanup_state.dart";
import "../../../library/domain/library_organization_cleanup.dart";
import "settings_section.dart";

String cleanupAgeDescription(LibraryCleanupInspection inspection) {
  final oldest = inspection.oldestUnassociatedUnixMs;
  final newest = inspection.newestUnassociatedUnixMs;
  if (oldest == null || newest == null) return "没有符合条件的未关联记录";
  final longest = Duration(
    milliseconds: inspection.inspectedUnixMs - oldest,
  ).inDays;
  final shortest = Duration(
    milliseconds: inspection.inspectedUnixMs - newest,
  ).inDays;
  if (longest < 1) return "未关联不足 1 天";
  if (shortest == longest) return "已未关联 $longest 天";
  return "未关联 ${shortest < 1 ? '不足 1' : shortest}～$longest 天";
}

Future<LibraryCleanupReady?> showOrganizationCleanupConfirmation(
  BuildContext context, {
  required int minimumAgeDays,
}) async {
  final container = ProviderScope.containerOf(context, listen: false);
  final controller = container.read(libraryCleanupControllerProvider.notifier);
  unawaited(controller.inspect(minimumAgeDays == 0 ? null : minimumAgeDays));
  LibraryCleanupReady? confirmed;
  try {
    confirmed = await showDialog<LibraryCleanupReady>(
      context: context,
      builder: (context) => _OrganizationCleanupConfirmation(
        initialMinimumAgeDays: minimumAgeDays,
      ),
    );
    return confirmed;
  } finally {
    if (confirmed == null) controller.cancel();
  }
}

class _OrganizationCleanupConfirmation extends ConsumerWidget {
  const _OrganizationCleanupConfirmation({required this.initialMinimumAgeDays});

  final int initialMinimumAgeDays;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(libraryCleanupControllerProvider);
    final controller = ref.read(libraryCleanupControllerProvider.notifier);
    final minimumAgeDays = switch (state) {
      LibraryCleanupReady(:final minimumAgeDays) => minimumAgeDays ?? 0,
      LibraryCleanupWorking(:final request) ||
      LibraryCleanupFailed(:final request) => request.minimumAgeDays ?? 0,
      _ => initialMinimumAgeDays,
    };
    final ready = state is LibraryCleanupReady ? state : null;
    return AlertDialog(
      key: const Key("organization-cleanup-confirmation"),
      title: const Text("清理未关联记录"),
      scrollable: true,
      content: SizedBox(
        width: 420,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SettingsChoice<int>(
              key: const Key("settings-cleanup-age"),
              value: minimumAgeDays,
              width: 260,
              enabled: state is! LibraryCleanupWorking,
              entries: const [
                SettingsChoiceEntry(value: 90, label: "未关联超过 90 天"),
                SettingsChoiceEntry(value: 30, label: "未关联超过 30 天"),
                SettingsChoiceEntry(value: 0, label: "全部未关联记录"),
              ],
              onSelected: (value) {
                if (value == null || value == minimumAgeDays) return;
                unawaited(controller.inspect(value == 0 ? null : value));
              },
            ),
            const SizedBox(height: 16),
            switch (state) {
              LibraryCleanupReady(:final inspection) => _inspection(inspection),
              LibraryCleanupWorking() => Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Semantics(
                    liveRegion: true,
                    child: Text(state.stopping ? "正在停止" : "正在检查未关联记录"),
                  ),
                  const SizedBox(height: 12),
                  const LinearProgressIndicator(),
                ],
              ),
              LibraryCleanupFailed() => const Text("未能完成检查，请重试。"),
              LibraryCleanupFinished() => const Text("检查已取消"),
              LibraryCleanupIdle() => const SizedBox.shrink(),
            },
          ],
        ),
      ),
      actions: [
        TextButton(
          autofocus: true,
          onPressed: () => Navigator.of(context).pop(),
          child: const Text("取消"),
        ),
        if (state is LibraryCleanupFailed)
          FilledButton(
            key: const Key("organization-cleanup-retry"),
            onPressed: () => unawaited(controller.retry()),
            child: const Text("重试"),
          )
        else
          FilledButton(
            key: const Key("organization-cleanup-confirm"),
            onPressed: ready != null && ready.inspection.total > 0
                ? () => Navigator.of(context).pop(ready)
                : null,
            child: const Text("清理记录"),
          ),
      ],
    );
  }

  Widget _inspection(LibraryCleanupInspection inspection) {
    if (inspection.total == 0) return const Text("没有符合条件的未关联记录");
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          "${inspection.groups} 组图片的记录 · ${inspection.duplicateChoices} 项去重选择",
        ),
        Text(cleanupAgeDescription(inspection)),
        const SizedBox(height: 16),
        Text("相册归属 ${inspection.albumGroups} 组"),
        Text("收藏 ${inspection.favoriteGroups} 组"),
        Text("整理进度 ${inspection.progressGroups} 组"),
        if (inspection.classificationGroups > 0)
          Text("分类记录 ${inspection.classificationGroups} 组"),
        const SizedBox(height: 16),
        const Text("相册和原图片会保留。重新导入图片不会恢复这些记录。"),
        const SizedBox(height: 8),
        const Text("本次清理无法撤销。如需恢复记录，可以手动恢复此前导出的备份。"),
      ],
    );
  }
}
