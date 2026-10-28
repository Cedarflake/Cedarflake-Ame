import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_album_pages.dart";
import "../../application/library_album_requests.dart";
import "../../application/library_organization_actions.dart";
import "../../application/library_sidebar_order.dart";
import "../../application/library_sidebar_album_removal.dart";
import "../../domain/library_gallery_collection.dart";
import "../../domain/library_organization.dart";
import "../library_sidebar_selection.dart";
import "library_album_editor.dart";
import "library_album_navigation_tile.dart";
import "library_sidebar_reorder_list.dart";
import "library_sidebar_section_header.dart";
import "library_sidebar_removal_dialog.dart";
import "library_sidebar_selection_region.dart";

class LibraryAlbumNavigation extends ConsumerStatefulWidget {
  const LibraryAlbumNavigation({
    required this.collection,
    required this.enabled,
    required this.onSelect,
    required this.header,
    this.isActive = true,
    super.key,
  });
  final LibraryGalleryCollection collection;
  final bool enabled;
  final ValueChanged<LibraryOrganizationAlbum> onSelect;
  final Widget header;
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
      title: "删除 ${albums.length} 个相册？",
      detail: "这些相册中的图片组将进入已忽略。图片文件和收藏状态会保留。",
      actionLabel: "删除相册",
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
        title: "相册",
        createLabel: "新建相册",
        createKey: const Key("library-sidebar-create-album"),
        selectedCount: _selection.count,
        selectAllLabel: page != null && page.total > page.albums.length
            ? "全选本页相册"
            : "全选相册",
        allSelected:
            page != null &&
            _selection.allSelected(page.albums.map((a) => a.id)),
        onSelectAll: busy || page == null
            ? null
            : () => setState(
                () => _selection.toggleAll(page.albums.map((a) => a.id)),
              ),
        removeLabel: "删除所选相册",
        onRemove: busy || page == null ? null : () => _removeSelected(page),
        onCreate: !widget.enabled || busy || page == null
            ? null
            : () => _edit(LibraryAlbumEditKind.create, page.revision),
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
        data: (value) => LibrarySidebarReorderList<LibraryOrganizationAlbum>(
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
          footer: (saving) => _previous.isEmpty && value.nextAlbumId == null
              ? const SizedBox.shrink()
              : Row(
                  children: [
                    TextButton(
                      onPressed:
                          saving ||
                              busy ||
                              _selection.isActive ||
                              _previous.isEmpty
                          ? null
                          : () =>
                                setState(() => _after = _previous.removeLast()),
                      child: const Text("上一页"),
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
                      child: const Text("下一页"),
                    ),
                  ],
                ),
        ),
        loading: () => ListView(
          padding: const EdgeInsets.fromLTRB(8, 12, 8, 12),
          children: [
            _header(null, true),
            const Padding(
              padding: EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: LinearProgressIndicator(),
            ),
          ],
        ),
        error: (error, stack) => ListView(
          padding: const EdgeInsets.fromLTRB(8, 12, 8, 12),
          children: [
            _header(null, true),
            ListTile(
              title: const Text("相册暂时无法加载"),
              trailing: TextButton(
                onPressed: () =>
                    ref.invalidate(libraryAlbumPageProvider(_after)),
                child: const Text("重试"),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
