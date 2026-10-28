import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../application/library_album_requests.dart";
import "../../domain/library_organization.dart";
import "library_sidebar_selection_icon.dart";

class LibraryAlbumNavigationTile extends StatelessWidget {
  const LibraryAlbumNavigationTile({
    required this.album,
    required this.selected,
    required this.onSelect,
    required this.onEdit,
    this.checked = false,
    this.onToggleSelection,
    super.key,
  });

  final LibraryOrganizationAlbum album;
  final bool selected;
  final VoidCallback? onSelect;
  final ValueChanged<LibraryAlbumEditKind>? onEdit;
  final bool checked;
  final VoidCallback? onToggleSelection;

  @override
  Widget build(BuildContext context) => ListTile(
    key: ValueKey("library-sidebar-album-${album.id}"),
    selected: selected,
    selectedTileColor: Theme.of(context).colorScheme.secondaryContainer,
    shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
    contentPadding: const EdgeInsets.only(left: 8, right: 4),
    minLeadingWidth: 40,
    horizontalTitleGap: 4,
    leading: LibrarySidebarSelectionIcon(
      key: ValueKey("library-sidebar-select-album-${album.id}"),
      icon: Symbols.photo_album_rounded,
      name: album.name,
      selected: checked,
      onPressed: onToggleSelection,
    ),
    title: Text(
      album.name,
      style: TextStyle(
        fontWeight: selected ? ameFontWeightSemibold : ameFontWeightMedium,
      ),
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
    ),
    onTap: onSelect,
    trailing: AmePopupMenuButton<LibraryAlbumEditKind>(
      labels: const ["重命名", "删除相册"],
      items: const [
        PopupMenuItem(
          value: LibraryAlbumEditKind.rename,
          child: AmeMenuItemContent(icon: Symbols.edit_rounded, label: "重命名"),
        ),
        PopupMenuItem(
          value: LibraryAlbumEditKind.delete,
          child: AmeMenuItemContent(
            icon: Symbols.delete_rounded,
            label: "删除相册",
          ),
        ),
      ],
      onSelected: (kind) => onEdit?.call(kind),
      builder: (context, openMenu) => AmeTooltip(
        message: "更多",
        child: IconButton(
          onPressed: onEdit == null ? null : openMenu,
          icon: const Icon(Symbols.more_vert_rounded),
        ),
      ),
    ),
  );
}
