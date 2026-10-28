import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_album_pages.dart";
import "../../application/library_album_requests.dart";
import "../../application/library_organization_actions.dart";
import "../../application/library_sidebar_order.dart";
import "../../domain/library_gallery_collection.dart";
import "../../domain/library_organization.dart";
import "library_album_editor.dart";
import "library_album_navigation_tile.dart";
import "library_sidebar_reorder_list.dart";
import "library_sidebar_section_header.dart";

class LibraryAlbumNavigation extends ConsumerStatefulWidget {
  const LibraryAlbumNavigation({
    required this.collection,
    required this.enabled,
    required this.onSelect,
    required this.header,
    super.key,
  });
  final LibraryGalleryCollection collection;
  final bool enabled;
  final ValueChanged<LibraryOrganizationAlbum> onSelect;
  final Widget header;

  @override
  ConsumerState<LibraryAlbumNavigation> createState() =>
      _LibraryAlbumNavigationState();
}

class _LibraryAlbumNavigationState
    extends ConsumerState<LibraryAlbumNavigation> {
  String? _after;
  final List<String?> _previous = [];

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
        onCreate: !widget.enabled || busy || page == null
            ? null
            : () => _edit(LibraryAlbumEditKind.create, page.revision),
      ),
    ],
  );

  @override
  Widget build(BuildContext context) {
    final page = ref.watch(libraryAlbumPageProvider(_after));
    final busy = ref.watch(
      libraryOrganizationActionsProvider.select((state) => state.saving),
    );
    return page.when(
      skipLoadingOnRefresh: true,
      data: (value) => LibrarySidebarReorderList<LibraryOrganizationAlbum>(
        key: ValueKey(_after),
        items: value.albums,
        itemId: (album) => album.id,
        enabled: widget.enabled && !busy,
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
                onSelect: widget.enabled ? () => widget.onSelect(album) : null,
                onEdit: busy
                    ? null
                    : (kind) => _edit(kind, value.revision, album),
              ),
            ),
        footer: (saving) => _previous.isEmpty && value.nextAlbumId == null
            ? const SizedBox.shrink()
            : Row(
                children: [
                  TextButton(
                    onPressed: saving || _previous.isEmpty
                        ? null
                        : () => setState(() => _after = _previous.removeLast()),
                    child: const Text("上一页"),
                  ),
                  const Spacer(),
                  TextButton(
                    onPressed: saving || value.nextAlbumId == null
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
              onPressed: () => ref.invalidate(libraryAlbumPageProvider(_after)),
              child: const Text("重试"),
            ),
          ),
        ],
      ),
    );
  }
}
