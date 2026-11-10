import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_file_operations.dart";
import "../../domain/library_file_operations.dart";
import "../../domain/library_organization.dart";
import "library_file_action_dialog.dart";

class LibraryDeleteFileButton extends ConsumerWidget {
  const LibraryDeleteFileButton({
    required this.target,
    this.enabled = true,
    this.style,
    this.buttonKey,
    super.key,
  });

  final LibraryOrganizationTarget target;
  final bool enabled;
  final ButtonStyle? style;
  final Key? buttonKey;

  @override
  Widget build(BuildContext context, WidgetRef ref) => AmeTooltip(
    message: "删除",
    child: IconButton(
      key: buttonKey,
      style: style,
      onPressed: !enabled || ref.watch(libraryFileActionsProvider).busy
          ? null
          : () => showLibraryFileAction(
              context: context,
              ref: ref,
              selection: LibraryFilesAtLocations([target]),
              action: LibraryFileAction.recycle,
            ),
      icon: const Icon(Symbols.delete_rounded, semanticLabel: "删除"),
    ),
  );
}
