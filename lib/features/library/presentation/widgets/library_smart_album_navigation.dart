import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_disclosure.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../../../app/presentation/ame_workbench_surfaces.dart";
import "../../application/library_classification_settings.dart";
import "../../application/library_classification_terms.dart";
import "../../application/library_model_installation.dart";
import "../../domain/library_classification_configuration.dart";
import "../../domain/library_gallery_collection.dart";
import "../library_category_label.dart";
import "library_model_setup.dart";
import "library_classification_terms_sheet.dart";
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
          title: context.l10n.smartAlbums,
          count: settings.hasValue
              ? settings.value?.visibleCategories.length ?? 0
              : null,
          action: IconButton(
            key: const Key("library-smart-albums-edit"),
            tooltip: context.l10n.editSmartAlbumTerms,
            style: IconButton.styleFrom(
              fixedSize: const Size.square(36),
              iconSize: 20,
              padding: const EdgeInsets.all(8),
              tapTargetSize: MaterialTapTargetSize.shrinkWrap,
            ),
            onPressed:
                ref.watch(libraryClassificationTermsServiceProvider) == null
                ? null
                : () => showLibraryClassificationTerms(context),
            icon: const Icon(Symbols.edit_rounded),
          ),
          trailing: LibrarySidebarSectionToggle(
            key: const Key("library-sidebar-smart-expand"),
            label: context.l10n.smartAlbums,
            expanded: _expanded,
            onPressed: () => setState(() => _expanded = !_expanded),
          ),
        ),
        AmeDisclosure(
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
                          title: Text(
                            context.l10n.noClassificationResults,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          trailing: value?.installationSelected == false
                              ? IconButton(
                                  tooltip: context.l10n.downloadModel,
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
                          title: Text(context.l10n.downloadModel),
                          onTap: canInstall
                              ? () => showLibraryModelSetup(context, ref)
                              : null,
                        ),
                    ],
                    loading: () => [
                      Padding(
                        padding: EdgeInsets.all(16),
                        child: Center(
                          child: SizedBox.square(
                            dimension: 24,
                            child: AmeCircularProgress(
                              strokeWidth: 2,
                              semanticsLabel: context.l10n.loadingSmartAlbums,
                            ),
                          ),
                        ),
                      ),
                    ],
                    error: (error, stack) => [
                      ListTile(
                        title: Text(context.l10n.categoriesUnavailable),
                        trailing: TextButton(
                          onPressed: () {
                            ref.invalidate(
                              libraryClassificationNavigationProvider,
                            );
                          },
                          child: Text(context.l10n.retry),
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
        libraryCategoryLabel(context.l10n, category),
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
