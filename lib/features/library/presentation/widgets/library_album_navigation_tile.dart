import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_popup_menu_position.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../application/library_album_requests.dart";
import "../../domain/library_organization.dart";
import "library_sidebar_selection_icon.dart";

class LibraryAlbumNavigationTile extends StatefulWidget {
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
  State<LibraryAlbumNavigationTile> createState() =>
      _LibraryAlbumNavigationTileState();
}

class _LibraryAlbumNavigationTileState
    extends State<LibraryAlbumNavigationTile> {
  final _focus = FocusNode(debugLabel: "Album row");
  final _menuAnchor = GlobalKey();
  bool _menuOpen = false;

  @override
  void dispose() {
    _focus.dispose();
    super.dispose();
  }

  void _openAtAnchor(BuildContext? anchor) {
    if (anchor == null) return;
    final position = amePopupMenuBelowAnchor(
      context: context,
      anchorContext: anchor,
    );
    if (position != null) unawaited(_showMenu(position));
  }

  Future<void> _showMenu(RelativeRect position) async {
    final onEdit = widget.onEdit;
    if (onEdit == null || _menuOpen) return;
    final albumId = widget.album.id;
    _menuOpen = true;
    try {
      final action = await showAmePopupMenu<LibraryAlbumEditKind>(
        context: context,
        position: position,
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
      );
      if (!mounted ||
          widget.album.id != albumId ||
          widget.onEdit == null ||
          action == null) {
        return;
      }
      onEdit(action);
    } finally {
      _menuOpen = false;
    }
  }

  @override
  Widget build(BuildContext context) => CallbackShortcuts(
    bindings: {
      const SingleActivator(LogicalKeyboardKey.contextMenu): () =>
          _openAtAnchor(_focus.context),
      const SingleActivator(LogicalKeyboardKey.f10, shift: true): () =>
          _openAtAnchor(_focus.context),
    },
    child: GestureDetector(
      behavior: HitTestBehavior.opaque,
      excludeFromSemantics: true,
      onSecondaryTapDown: (details) {
        final position = amePopupMenuAtGlobalPosition(
          context: context,
          globalPosition: details.globalPosition,
        );
        if (position != null) unawaited(_showMenu(position));
      },
      child: ListTile(
        key: ValueKey("library-sidebar-album-${widget.album.id}"),
        focusNode: _focus,
        selected: widget.selected,
        selectedTileColor: Theme.of(context).colorScheme.secondaryContainer,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        contentPadding: const EdgeInsets.only(left: 8, right: 4),
        minLeadingWidth: 40,
        horizontalTitleGap: 4,
        leading: LibrarySidebarSelectionIcon(
          key: ValueKey("library-sidebar-select-album-${widget.album.id}"),
          icon: Symbols.photo_album_rounded,
          name: widget.album.name,
          selected: widget.checked,
          onPressed: widget.onToggleSelection,
        ),
        title: Text(
          widget.album.name,
          style: TextStyle(
            fontWeight: widget.selected
                ? ameFontWeightSemibold
                : ameFontWeightMedium,
          ),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
        ),
        onTap: widget.onSelect,
        trailing: SizedBox(
          key: _menuAnchor,
          child: AmeTooltip(
            message: "更多",
            child: IconButton(
              key: ValueKey("library-sidebar-album-menu-${widget.album.id}"),
              onPressed: widget.onEdit == null
                  ? null
                  : () => _openAtAnchor(_menuAnchor.currentContext),
              icon: const Icon(Symbols.more_vert_rounded),
            ),
          ),
        ),
      ),
    ),
  );
}
