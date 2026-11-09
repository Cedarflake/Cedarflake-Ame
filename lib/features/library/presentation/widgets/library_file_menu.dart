import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../application/library_file_operations.dart";
import "../../domain/library_file_operations.dart";
import "../library_file_strings.dart";
import "library_file_action_dialog.dart";

class LibraryFileMenu extends ConsumerWidget {
  const LibraryFileMenu({
    required this.selection,
    required this.enabled,
    super.key,
  });
  final LibraryFileSelection selection;
  final bool enabled;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final busy = ref.watch(libraryFileActionsProvider).busy;
    const actions = [
      LibraryFileAction.copy,
      LibraryFileAction.move,
      LibraryFileAction.recycle,
    ];
    return AmePopupMenuButton<LibraryFileAction>(
      labels: [for (final action in actions) LibraryFileStrings.action(action)],
      items: [
        for (final action in actions)
          PopupMenuItem(
            value: action,
            child: AmeMenuItemContent(
              label: LibraryFileStrings.action(action),
              icon: switch (action) {
                LibraryFileAction.copy => Symbols.content_copy_rounded,
                LibraryFileAction.move => Symbols.drive_file_move_rounded,
                _ => Symbols.delete_rounded,
              },
            ),
          ),
      ],
      onSelected: (action) => showLibraryFileAction(
        context: context,
        ref: ref,
        selection: selection,
        action: action,
      ),
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
