import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_disclosure.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../application/library_album_pages.dart";
import "../../application/library_album_requests.dart";
import "../../application/library_organization_actions.dart";
import "../../application/library_sidebar_order.dart";
import "../../domain/library_sidebar_order.dart";
import "../../application/library_sidebar_album_removal.dart";
import "../../domain/library_gallery_collection.dart";
import "../../domain/library_organization.dart";
import "../coordination/library_sidebar_selection.dart";
import "library_album_editor.dart";
import "library_album_icon.dart";
import "library_album_navigation_tile.dart";
import "library_sidebar_reorder_list.dart";
import "library_sidebar_section_header.dart";
import "library_sidebar_section_toggle.dart";
import "library_sidebar_rows_disclosure.dart";
import "library_sidebar_removal_dialog.dart";
import "library_sidebar_selection_region.dart";
import "library_workbench_metrics.dart";

class LibraryAlbumNavigation extends ConsumerStatefulWidget {
  const LibraryAlbumNavigation({
    required this.collection,
    required this.enabled,
    required this.onSelect,
    required this.header,
    this.footer,
    this.isActive = true,
    super.key,
  });
  final LibraryGalleryCollection collection;
  final bool enabled;
  final ValueChanged<LibraryOrganizationAlbum> onSelect;
  final Widget header;
  final Widget? footer;
  final bool isActive;

  @override
  ConsumerState<LibraryAlbumNavigation> createState() =>
      _LibraryAlbumNavigationState();
}

class _LibraryAlbumNavigationState
    extends ConsumerState<LibraryAlbumNavigation> {
  String? _after;
  final List<String?> _previous = [];
  final _selection = LibrarySidebarSelection();
  bool _removing = false;
  bool _expanded = true;

  @override
  void didUpdateWidget(LibraryAlbumNavigation oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!widget.isActive) _selection.clear();
  }

  Future<void> _removeSelected(LibraryOrganizationAlbumPage page) async {
    final albums = page.albums
        .where((album) => _selection.contains(album.id))
        .toList();
    if (albums.isEmpty || _removing) return;
    setState(() => _removing = true);
    final completed = await showLibrarySidebarRemovalDialog(
      context: context,
      removal: createSidebarAlbumRemoval(
        albums: albums,
        revision: page.revision,
        actions: ref.read(libraryOrganizationActionsProvider.notifier),
      ),
      title: context.l10n.confirmDeleteAlbums(albums.length),
      detail: context.l10n.deleteAlbumsImpact,
      actionLabel: context.l10n.deleteAlbum,
      itemIcons: {
        for (final album in albums) album.id: libraryAlbumIcon(album.icon),
      },
    );
    if (!mounted) return;
    setState(() {
      _removing = false;
      _selection.removeAll(completed);
    });
  }

  Future<void> _edit(
    LibraryAlbumEditKind kind,
    BigInt revision, [
    LibraryOrganizationAlbum? album,
  ]) async {
    await showLibraryAlbumEditor(
      context: context,
      kind: kind,
      revision: revision,
      album: album,
    );
  }

  Widget _header(LibraryOrganizationAlbumPage? page, bool busy) => Column(
    children: [
      widget.header,
      LibrarySidebarSectionHeader(
        title: context.l10n.albums,
        count: page?.total,
        createLabel: context.l10n.createAlbum,
        createKey: const Key("library-sidebar-create-album"),
        onCreate: !widget.enabled || busy || page == null
            ? null
            : () => _edit(LibraryAlbumEditKind.create, page.revision),
        trailing: LibrarySidebarSectionToggle(
          key: const Key("library-sidebar-album-expand"),
          label: context.l10n.albums,
          expanded: _expanded,
          onPressed: () => setState(() => _expanded = !_expanded),
        ),
        selectedCount: _selection.count,
        selectAllLabel: page != null && page.total > page.albums.length
            ? context.l10n.selectPageAlbums
            : context.l10n.selectAllAlbums,
        allSelected:
            page != null &&
            _selection.allSelected(page.albums.map((a) => a.id)),
        onSelectAll: busy || page == null
            ? null
            : () => setState(
                () => _selection.toggleAll(page.albums.map((a) => a.id)),
              ),
        removeLabel: context.l10n.deleteSelectedAlbums,
        onRemove: busy || page == null ? null : () => _removeSelected(page),
      ),
      AmeDisclosure(
        child: _expanded && page?.total == 0
            ? ListTile(
                key: Key("library-sidebar-album-empty"),
                minTileHeight: LibraryWorkbenchMetrics.sidebarRowHeight,
                contentPadding: EdgeInsets.only(left: 16, right: 6),
                minLeadingWidth: 24,
                horizontalTitleGap: 12,
                leading: Icon(Symbols.photo_album_rounded, size: 24),
                title: Text(context.l10n.albumsEmpty),
              )
            : null,
      ),
    ],
  );

  @override
  Widget build(BuildContext context) {
    final page = ref.watch(libraryAlbumPageProvider(_after));
    final saving = ref.watch(
      libraryOrganizationActionsProvider.select((state) => state.saving),
    );
    final busy = _removing || saving;
    final loaded = page.asData?.value;
    if (loaded != null && !_removing) {
      _selection.retain(loaded.albums.map((album) => album.id));
    }
    return LibrarySidebarSelectionRegion(
      onClear: _selection.isActive && !_removing
          ? () => setState(_selection.clear)
          : null,
      child: page.when(
        skipLoadingOnRefresh: true,
        skipLoadingOnReload: true,
        data: (value) => LibrarySidebarRowsDisclosure(
          expanded: _expanded,
          child: LibrarySidebarReorderList<LibraryOrganizationAlbum>(
            key: ValueKey(_after),
            items: value.albums,
            itemId: (album) => album.id,
            enabled: widget.enabled && !busy && !_selection.isActive,
            onReorder: (move) => ref
                .read(librarySidebarOrderProvider)
                .move(LibrarySidebarKind.albums, move),
            onSaved: () => ref.invalidate(libraryAlbumPageProvider),
            header: (saving) => _header(value, busy || saving),
            itemBuilder: (context, album, index, draggable) =>
                ReorderableDragStartListener(
                  index: index,
                  enabled: draggable,
                  child: LibraryAlbumNavigationTile(
                    album: album,
                    selected:
                        widget.collection ==
                        LibraryGalleryCollection.album(album.id),
                    checked: _selection.contains(album.id),
                    onToggleSelection: !widget.enabled || busy
                        ? null
                        : () => setState(() => _selection.toggle(album.id)),
                    onSelect: !widget.enabled || busy
                        ? null
                        : () {
                            if (_selection.isActive) {
                              setState(() => _selection.toggle(album.id));
                            } else {
                              widget.onSelect(album);
                            }
                          },
                    onEdit: busy || _selection.isActive
                        ? null
                        : (kind) => _edit(kind, value.revision, album),
                  ),
                ),
            footer: (saving) => Column(
              children: [
                if (_expanded &&
                    (_previous.isNotEmpty || value.nextAlbumId != null))
                  Row(
                    children: [
                      TextButton(
                        onPressed:
                            saving ||
                                busy ||
                                _selection.isActive ||
                                _previous.isEmpty
                            ? null
                            : () => setState(
                                () => _after = _previous.removeLast(),
                              ),
                        child: Text(context.l10n.previousPage),
                      ),
                      const Spacer(),
                      TextButton(
                        onPressed:
                            saving ||
                                busy ||
                                _selection.isActive ||
                                value.nextAlbumId == null
                            ? null
                            : () => setState(() {
                                _previous.add(_after);
                                _after = value.nextAlbumId;
                              }),
                        child: Text(context.l10n.nextPage),
                      ),
                    ],
                  ),
                if (widget.footer != null) widget.footer!,
              ],
            ),
          ),
        ),
        loading: () => ListView(
          padding: const EdgeInsets.fromLTRB(16, 20, 16, 12),
          children: [
            _header(null, true),
            AmeDisclosure(
              child: _expanded
                  ? const Padding(
                      padding: EdgeInsets.symmetric(
                        horizontal: 16,
                        vertical: 8,
                      ),
                      child: AmeLinearProgress(),
                    )
                  : null,
            ),
            if (widget.footer != null) widget.footer!,
          ],
        ),
        error: (error, stack) => ListView(
          padding: const EdgeInsets.fromLTRB(16, 20, 16, 12),
          children: [
            _header(null, true),
            AmeDisclosure(
              child: _expanded
                  ? ListTile(
                      title: Text(context.l10n.albumsUnavailable),
                      trailing: TextButton(
                        onPressed: () =>
                            ref.invalidate(libraryAlbumPageProvider(_after)),
                        child: Text(context.l10n.retry),
                      ),
                    )
                  : null,
            ),
            if (widget.footer != null) widget.footer!,
          ],
        ),
      ),
    );
  }
}
