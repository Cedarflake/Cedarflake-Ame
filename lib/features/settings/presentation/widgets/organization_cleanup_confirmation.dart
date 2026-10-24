import "package:flutter/material.dart";

import "../../../library/domain/library_organization_cleanup.dart";

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

Future<bool> showOrganizationCleanupConfirmation(
  BuildContext context,
  LibraryCleanupInspection inspection,
) async =>
    await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        key: const Key("organization-cleanup-confirmation"),
        title: const Text("清理未关联记录"),
        scrollable: true,
        content: SizedBox(
          width: 420,
          child: Column(
            mainAxisSize: MainAxisSize.min,
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
              const SizedBox(height: 16),
              const Text("相册和原图片会保留。重新导入图片不会恢复这些记录。"),
              const SizedBox(height: 8),
              const Text("本次清理无法撤销。如需恢复记录，可以手动恢复此前导出的备份。"),
            ],
          ),
        ),
        actions: [
          TextButton(
            autofocus: true,
            onPressed: () => Navigator.of(context).pop(false),
            child: const Text("取消"),
          ),
          FilledButton(
            key: const Key("organization-cleanup-confirm"),
            onPressed: () => Navigator.of(context).pop(true),
            child: const Text("清理记录"),
          ),
        ],
      ),
    ) ??
    false;
