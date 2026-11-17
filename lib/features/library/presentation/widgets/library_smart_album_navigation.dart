import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../../../app/presentation/ame_workbench_surfaces.dart";
import "../../application/library_classification_settings.dart";
import "../../application/library_model_installation.dart";
import "../../domain/library_classification_configuration.dart";
import "../../domain/library_gallery_collection.dart";
import "library_model_setup.dart";
import "library_folder_disclosure.dart";
import "library_sidebar_section_header.dart";
import "library_sidebar_section_toggle.dart";
import "library_workbench_metrics.dart";

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
    final settings = ref.watch(libraryClassificationNavigationProvider);
    final canInstall = ref.watch(
      libraryModelInstallationProvider.select(
        (installation) => installation?.canStart == true,
      ),
    );
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        LibrarySidebarSectionHeader(
          key: const Key("library-sidebar-smart-heading"),
          title: "智能相册",
          count: settings.hasValue
              ? settings.value?.visibleCategories.length ?? 0
              : null,
          trailing: LibrarySidebarSectionToggle(
            key: const Key("library-sidebar-smart-expand"),
            label: "智能相册",
            expanded: _expanded,
            onPressed: () => setState(() => _expanded = !_expanded),
          ),
        ),
        LibraryFolderDisclosure(
          child: _expanded
              ? Column(
                  children: settings.when(
                    skipLoadingOnReload: true,
                    data: (value) => [
                      if (value == null || value.visibleCategories.isEmpty)
                        ListTile(
                          key: const Key("library-sidebar-smart-empty"),
                          minTileHeight:
                              LibraryWorkbenchMetrics.sidebarRowHeight,
                          contentPadding: const EdgeInsets.only(
                            left: 16,
                            right: 4,
                          ),
                          leading: const Icon(
                            Symbols.photo_album_rounded,
                            size: 24,
                          ),
                          minLeadingWidth: 24,
                          horizontalTitleGap: 12,
                          title: const Text(
                            "还没有分类结果",
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          trailing: value?.installationSelected == false
                              ? IconButton(
                                  tooltip: "下载模型",
                                  style: IconButton.styleFrom(
                                    fixedSize: const Size.square(36),
                                    iconSize: 20,
                                    padding: const EdgeInsets.all(8),
                                    tapTargetSize:
                                        MaterialTapTargetSize.shrinkWrap,
                                  ),
                                  onPressed: canInstall
                                      ? () =>
                                            showLibraryModelSetup(context, ref)
                                      : null,
                                  icon: const Icon(Symbols.download_rounded),
                                )
                              : null,
                        ),
                      for (final category
                          in value?.visibleCategories ??
                              const <LibraryClassificationCategory>[])
                        _SmartAlbumRow(
                          category: category,
                          selected:
                              widget.collection
                                  is LibrarySmartAlbumCollection &&
                              (widget.collection as LibrarySmartAlbumCollection)
                                      .categoryId ==
                                  category.id,
                          onSelect: widget.enabled
                              ? () => widget.onSelect(category)
                              : null,
                        ),
                      if (value?.installationSelected == false &&
                          value!.visibleCategories.isNotEmpty)
                        ListTile(
                          leading: const Icon(Symbols.download_rounded),
                          title: const Text("下载模型"),
                          onTap: canInstall
                              ? () => showLibraryModelSetup(context, ref)
                              : null,
                        ),
                    ],
                    loading: () => const [
                      Padding(
                        padding: EdgeInsets.all(16),
                        child: Center(
                          child: SizedBox.square(
                            dimension: 24,
                            child: AmeCircularProgress(
                              strokeWidth: 2,
                              semanticsLabel: "正在加载智能相册",
                            ),
                          ),
                        ),
                      ),
                    ],
                    error: (error, stack) => [
                      ListTile(
                        title: const Text("分类暂时无法加载"),
                        trailing: TextButton(
                          onPressed: () {
                            ref.invalidate(
                              libraryClassificationNavigationProvider,
                            );
                          },
                          child: const Text("重试"),
                        ),
                      ),
                    ],
                  ),
                )
              : null,
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
      selectedTileColor: AmeWorkbenchSurfaces.of(context).selection,
      selectedColor: AmeWorkbenchSurfaces.of(context).onSelection,
      shape: LibraryWorkbenchMetrics.sidebarRowShape,
      minTileHeight: LibraryWorkbenchMetrics.sidebarRowHeight,
      minVerticalPadding: 6,
      contentPadding: const EdgeInsets.only(left: 16, right: 16),
      minLeadingWidth: 24,
      horizontalTitleGap: 12,
      leading: SizedBox(
        width: 24,
        child: Icon(switch (category.id) {
          "photo" => Symbols.photo_camera_rounded,
          "illustration" => Symbols.palette_rounded,
          "comic" => Symbols.menu_book_rounded,
          "meme" => Symbols.mood_rounded,
          "screenshot" => Symbols.desktop_windows_rounded,
          _ => Symbols.photo_album_rounded,
        }, size: 24),
      ),
      title: Text(
        category.label,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
        style: theme.textTheme.bodyLarge?.copyWith(
          fontWeight: selected ? ameFontWeightSemibold : ameFontWeightRegular,
        ),
      ),
      onTap: onSelect,
    );
  }
}
