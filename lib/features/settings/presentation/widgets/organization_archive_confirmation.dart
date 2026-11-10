import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_notification_time.dart";
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
        builder: (context) => AmeDialog(
          key: const Key("organization-archive-confirmation"),
          title: "恢复整理记录",
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              AmeDialogItem(
                icon: Symbols.settings_backup_restore_rounded,
                title: archiveFilename(ready.path),
                subtitle: created == null
                    ? null
                    : "创建时间：${formatAmeNotificationOccurredAt(created)}",
              ),
              const SizedBox(height: 24),
              for (final row in [
                if (data.albums > 0)
                  (Symbols.photo_album_rounded, "${data.albums} 个相册"),
                if (data.groups > 0)
                  (Symbols.photo_library_rounded, "${data.groups} 组图片"),
                if (data.favorites > 0)
                  (Symbols.favorite_rounded, "${data.favorites} 组收藏"),
                if (data.duplicateChoices > 0)
                  (Symbols.layers_rounded, "${data.duplicateChoices} 项去重选择"),
                if (data.matchedGroups > 0)
                  (Symbols.link_rounded, "可关联 ${data.matchedGroups} 组"),
                if (data.unmatchedGroups > 0)
                  (Symbols.link_off_rounded, "暂未关联 ${data.unmatchedGroups} 组"),
                if (data.conflicts > 0)
                  (Symbols.warning_rounded, "${data.conflicts} 组相册冲突"),
                if (data.unresolvedPreferredCopies > 0)
                  (
                    Symbols.image_search_rounded,
                    "${data.unresolvedPreferredCopies} 项首选副本暂无法关联",
                  ),
                if (data.hasContinuation)
                  (Symbols.bookmark_rounded, "包含继续整理的位置"),
              ])
                Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: AmeDialogItem(icon: row.$1, title: row.$2),
                ),
              const SizedBox(height: 8),
              const Text("当前选择和整理位置会保留，不会移动或修改原图片。"),
              if (data.unmatchedGroups > 0 ||
                  data.unresolvedPreferredCopies > 0) ...[
                const SizedBox(height: 8),
                const Text("暂未关联的记录会等待图片重新关联。"),
              ],
            ],
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
