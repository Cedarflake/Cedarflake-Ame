import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_dialog_location.dart";
import "../../../library/application/library_platform_actions.dart";

class StoragePreviewCleanupConfirmation extends ConsumerWidget {
  const StoragePreviewCleanupConfirmation({
    this.retiredDirectory,
    this.retiredDisplayPath,
    super.key,
  });

  final String? retiredDirectory;
  final String? retiredDisplayPath;

  @override
  Widget build(BuildContext context, WidgetRef ref) => AmeDialog(
    title: retiredDirectory == null ? "清理缩略图？" : "清理旧缩略图目录？",
    content: Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        if (retiredDirectory case final path?) ...[
          AmeDialogLocation(
            label: "旧缩略图文件夹",
            path: retiredDisplayPath ?? path,
            onOpen: () =>
                ref.read(libraryPlatformActionsProvider).revealDirectory(path),
          ),
          const SizedBox(height: 20),
        ],
        Text(
          retiredDirectory == null
              ? "这会删除可重新生成的缩略图缓存，不会删除或修改原图片。清理后，打开图库时可能需要一些时间重新生成缩略图。"
              : "只会删除这个文件夹中由 Ame 管理的缩略图。原图片和其他文件会保留。",
        ),
      ],
    ),
    actions: [
      TextButton(
        onPressed: () => Navigator.pop(context, false),
        child: const Text("取消"),
      ),
      FilledButton(
        onPressed: () => Navigator.pop(context, true),
        child: const Text("开始清理"),
      ),
    ],
  );
}
