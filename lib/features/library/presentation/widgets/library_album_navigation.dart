import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_album_pages.dart";
import "../../application/library_album_requests.dart";
import "../../application/library_organization_actions.dart";
import "../../domain/library_gallery_collection.dart";
import "../../domain/library_organization.dart";
import "library_album_editor.dart";

class LibraryAlbumNavigation extends ConsumerStatefulWidget {
  const LibraryAlbumNavigation({
    required this.collection,
    required this.enabled,
    required this.onSelect,
    super.key,
  });
  final LibraryGalleryCollection collection;
  final bool enabled;
  final ValueChanged<LibraryOrganizationAlbum> onSelect;

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

  @override
  Widget build(BuildContext context) {
    final page = ref.watch(libraryAlbumPageProvider(_after));
    final busy = ref.watch(
      libraryOrganizationActionsProvider.select((state) => state.saving),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.only(left: 16, top: 8),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  "相册",
                  style: Theme.of(context).textTheme.titleSmall,
                ),
              ),
              AmeTooltip(
                message: "新建相册",
                child: IconButton(
                  key: const Key("library-sidebar-create-album"),
                  onPressed:
                      !widget.enabled ||
                          busy ||
                          page.isLoading ||
                          page.value == null
                      ? null
                      : () => _edit(
                          LibraryAlbumEditKind.create,
                          page.value!.revision,
                        ),
                  icon: const Icon(Symbols.add_rounded, size: 20),
                ),
              ),
            ],
          ),
        ),
        page.when(
          skipLoadingOnRefresh: true,
          data: (value) => Column(
            children: [
              for (final album in value.albums)
                ListTile(
                  key: ValueKey("library-sidebar-album-${album.id}"),
                  selected:
                      widget.collection ==
                      LibraryGalleryCollection.album(album.id),
                  selectedTileColor: Theme.of(
                    context,
                  ).colorScheme.secondaryContainer,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(12),
                  ),
                  contentPadding: const EdgeInsets.only(left: 16, right: 4),
                  minLeadingWidth: 24,
                  horizontalTitleGap: 12,
                  leading: const SizedBox(
                    width: 24,
                    child: Icon(Symbols.photo_album_rounded),
                  ),
                  title: Text(
                    album.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  onTap: widget.enabled ? () => widget.onSelect(album) : null,
                  trailing: AmePopupMenuButton<LibraryAlbumEditKind>(
                    labels: const ["重命名", "删除相册"],
                    items: const [
                      PopupMenuItem(
                        value: LibraryAlbumEditKind.rename,
                        child: AmeMenuItemContent(
                          icon: Symbols.edit_rounded,
                          label: "重命名",
                        ),
                      ),
                      PopupMenuItem(
                        value: LibraryAlbumEditKind.delete,
                        child: AmeMenuItemContent(
                          icon: Symbols.delete_rounded,
                          label: "删除相册",
                        ),
                      ),
                    ],
                    onSelected: (kind) => _edit(kind, value.revision, album),
                    builder: (context, openMenu) => AmeTooltip(
                      message: "更多",
                      child: IconButton(
                        onPressed: busy ? null : openMenu,
                        icon: const Icon(Symbols.more_vert_rounded, size: 20),
                      ),
                    ),
                  ),
                ),
              if (_previous.isNotEmpty || value.nextAlbumId != null)
                Row(
                  children: [
                    TextButton(
                      onPressed: _previous.isEmpty
                          ? null
                          : () =>
                                setState(() => _after = _previous.removeLast()),
                      child: const Text("上一页"),
                    ),
                    const Spacer(),
                    TextButton(
                      onPressed: value.nextAlbumId == null
                          ? null
                          : () => setState(() {
                              _previous.add(_after);
                              _after = value.nextAlbumId;
                            }),
                      child: const Text("下一页"),
                    ),
                  ],
                ),
            ],
          ),
          loading: () => const Padding(
            padding: EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: LinearProgressIndicator(),
          ),
          error: (error, stack) => ListTile(
            title: const Text("相册暂时无法加载"),
            trailing: TextButton(
              onPressed: () => ref.invalidate(libraryAlbumPageProvider(_after)),
              child: const Text("重试"),
            ),
          ),
        ),
      ],
    );
  }
}
