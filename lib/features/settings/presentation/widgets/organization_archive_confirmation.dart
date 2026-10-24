import "package:flutter/material.dart";

import "../../../library/application/library_archive_state.dart";
import "../organization_archive_messages.dart";

Future<bool> showOrganizationArchiveConfirmation(
  BuildContext context,
  LibraryArchiveReady ready,
) async {
  final data = ready.inspection;
  final created =
      data.createdUnixMs >= BigInt.zero &&
          data.createdUnixMs <= BigInt.from(8640000000000000)
      ? DateTime.fromMillisecondsSinceEpoch(data.createdUnixMs.toInt())
      : null;
  return await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          key: const Key("organization-archive-confirmation"),
          title: const Text("恢复整理记录"),
          scrollable: true,
          content: SizedBox(
            width: 420,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(archiveFilename(ready.path)),
                if (created != null)
                  Text(
                    "创建于 ${MaterialLocalizations.of(context).formatCompactDate(created)}",
                  ),
                const SizedBox(height: 16),
                Text(
                  "${data.albums} 个相册 · ${data.groups} 组图片 · ${data.favorites} 组收藏",
                ),
                Text(
                  "可关联 ${data.matchedGroups} 组，暂未关联 ${data.unmatchedGroups} 组",
                ),
                if (data.conflicts > 0) Text("备份包含 ${data.conflicts} 组相册冲突"),
                if (data.duplicateChoices > 0)
                  Text("${data.duplicateChoices} 项去重选择"),
                if (data.unresolvedPreferredCopies > 0)
                  Text("${data.unresolvedPreferredCopies} 项首选副本暂无法关联"),
                if (data.hasContinuation) const Text("包含继续整理的位置"),
                const SizedBox(height: 16),
                const Text("当前已作出的选择和整理位置会保留。暂未关联的记录会等待图片重新关联。"),
                const SizedBox(height: 8),
                const Text("不会移动或修改原图片。"),
              ],
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text("取消"),
            ),
            FilledButton(
              key: const Key("organization-archive-restore-confirm"),
              onPressed: () => Navigator.of(context).pop(true),
              child: const Text("恢复"),
            ),
          ],
        ),
      ) ??
      false;
}
