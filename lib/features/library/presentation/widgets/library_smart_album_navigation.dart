import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_typography.dart";
import "../../application/library_classification_configuration.dart";
import "../../application/library_classification_settings.dart";
import "../../domain/library_gallery_collection.dart";

class LibrarySmartAlbumNavigation extends ConsumerStatefulWidget {
  const LibrarySmartAlbumNavigation({
    required this.collection,
    required this.enabled,
    required this.onSelect,
    super.key,
  });

  final LibraryGalleryCollection collection;
  final bool enabled;
  final ValueChanged<LibraryClassificationCategory> onSelect;

  @override
  ConsumerState<LibrarySmartAlbumNavigation> createState() =>
      _LibrarySmartAlbumNavigationState();
}

class _LibrarySmartAlbumNavigationState
    extends ConsumerState<LibrarySmartAlbumNavigation> {
  bool _expanded = true;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final settings = ref.watch(libraryClassificationSettingsProvider);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(16, 12, 6, 4),
          child: Row(
            children: [
              Expanded(
                child: Text(
                  "智能相册",
                  key: const Key("library-sidebar-smart-heading"),
                  style: theme.textTheme.titleSmall?.copyWith(
                    fontWeight: ameFontWeightSemibold,
                  ),
                ),
              ),
              Semantics(
                expanded: _expanded,
                child: IconButton(
                  key: const Key("library-sidebar-smart-expand"),
                  tooltip: _expanded ? "收起智能相册" : "展开智能相册",
                  onPressed: () => setState(() => _expanded = !_expanded),
                  style: IconButton.styleFrom(
                    fixedSize: const Size.square(36),
                    iconSize: 20,
                    padding: const EdgeInsets.all(8),
                    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  ),
                  icon: Icon(
                    _expanded
                        ? Symbols.expand_less_rounded
                        : Symbols.expand_more_rounded,
                  ),
                ),
              ),
            ],
          ),
        ),
        if (_expanded)
          ...settings.when(
            skipLoadingOnReload: true,
            data: (value) => [
              if (value == null || value.categories.isEmpty)
                const ListTile(
                  key: Key("library-sidebar-smart-empty"),
                  leading: Icon(Symbols.photo_album_rounded),
                  title: Text("还没有分类结果"),
                ),
              for (final category in value?.categories ?? const [])
                _SmartAlbumRow(
                  category: category,
                  selected:
                      widget.collection is LibrarySmartAlbumCollection &&
                      (widget.collection as LibrarySmartAlbumCollection)
                              .categoryId ==
                          category.id,
                  onSelect: widget.enabled
                      ? () => widget.onSelect(category)
                      : null,
                ),
            ],
            loading: () => const <Widget>[],
            error: (error, stack) => [
              ListTile(
                title: const Text("分类暂时无法加载"),
                trailing: TextButton(
                  onPressed: () {
                    ref.invalidate(libraryClassificationSettingsProvider);
                  },
                  child: const Text("重试"),
                ),
              ),
            ],
          ),
      ],
    );
  }
}

class _SmartAlbumRow extends StatelessWidget {
  const _SmartAlbumRow({
    required this.category,
    required this.selected,
    required this.onSelect,
  });

  final LibraryClassificationCategory category;
  final bool selected;
  final VoidCallback? onSelect;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListTile(
      key: ValueKey("library-sidebar-smart-${category.id}"),
      selected: selected,
      selectedTileColor: theme.colorScheme.secondaryContainer,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(10)),
      minTileHeight: 40,
      minVerticalPadding: 6,
      contentPadding: const EdgeInsets.symmetric(horizontal: 16),
      minLeadingWidth: 24,
      horizontalTitleGap: 12,
      leading: Icon(switch (category.id) {
        "photo" => Symbols.photo_camera_rounded,
        "illustration" => Symbols.palette_rounded,
        "comic" => Symbols.menu_book_rounded,
        "meme" => Symbols.mood_rounded,
        "screenshot" => Symbols.desktop_windows_rounded,
        "other" => Symbols.folder_rounded,
        _ => Symbols.photo_album_rounded,
      }, size: 20),
      title: Text(
        category.label,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: theme.textTheme.bodyMedium?.copyWith(
          fontWeight: selected ? ameFontWeightSemibold : ameFontWeightMedium,
        ),
      ),
      onTap: onSelect,
    );
  }
}
