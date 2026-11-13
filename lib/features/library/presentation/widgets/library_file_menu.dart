import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../application/library_file_operations.dart";
import "../../application/library_file_selection_scope.dart";
import "../../domain/library_file_operations.dart";
import "../../domain/library_organization_batch.dart";
import "../../domain/library_gallery_group.dart";
import "../library_file_strings.dart";
import "library_file_action_dialog.dart";

class LibraryFileMenu extends ConsumerWidget {
  const LibraryFileMenu({
    required this.selection,
    required this.enabled,
    required this.onCommitted,
    super.key,
  });
  final LibraryOrganizationSelection selection;
  final bool enabled;
  final VoidCallback onCommitted;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final busy = ref.watch(libraryFileActionsProvider).busy;
    final actions = [
      _FileMenuAction.copy,
      _FileMenuAction.move,
      _FileMenuAction.delete,
      if (selection.query.hasGroupFilter) _FileMenuAction.cleanup,
    ];
    String label(_FileMenuAction action) => action == _FileMenuAction.cleanup
        ? (selection.query.groupFilter == LibraryGalleryGroupFilter.similar
              ? "删除相似图片"
              : "删除重复图片")
        : LibraryFileStrings.action(action.fileAction);
    return AmePopupMenuButton<_FileMenuAction>(
      labels: [for (final action in actions) label(action)],
      items: [
        for (final action in actions)
          PopupMenuItem(
            value: action,
            child: AmeMenuItemContent(
              label: label(action),
              icon: switch (action) {
                _FileMenuAction.copy => Symbols.content_copy_rounded,
                _FileMenuAction.move => Symbols.drive_file_move_rounded,
                _ => Symbols.delete_rounded,
              },
            ),
          ),
      ],
      onSelected: (action) async {
        final submitted = await showLibraryFileAction(
          context: context,
          ref: ref,
          selection: action == _FileMenuAction.cleanup
              ? LibraryCleanupInGallery(selection)
              : selectGalleryFilesForAction(
                  action: action.fileAction,
                  selection: selection,
                ),
          action: action.fileAction,
          initialRule: initialFileOrganizationRule(
            action: action.fileAction,
            query: selection.query,
          ),
        );
        final result = await submitted?.completion;
        if (context.mounted && (result?.published ?? 0) > 0) onCommitted();
      },
      builder: (context, openMenu) => TextButton.icon(
        onPressed: enabled && !busy ? openMenu : null,
        icon: const Icon(Symbols.folder_rounded),
        label: const Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text("文件"),
            SizedBox(width: 4),
            Icon(Symbols.arrow_drop_down_rounded, size: 20),
          ],
        ),
      ),
    );
  }
}

enum _FileMenuAction {
  copy,
  move,
  delete,
  cleanup;

  LibraryFileAction get fileAction => switch (this) {
    copy => LibraryFileAction.copy,
    move => LibraryFileAction.move,
    delete || cleanup => LibraryFileAction.recycle,
  };
}
