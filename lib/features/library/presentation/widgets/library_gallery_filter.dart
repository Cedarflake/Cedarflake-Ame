import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_models.dart";
import "../library_duplicate_strings.dart";
import "library_metadata_filter_editor.dart";

enum _FilterChoice { descendants, folderOnly, all, merged, metadata }

class LibraryGalleryFilter extends StatelessWidget {
  const LibraryGalleryFilter({
    required this.query,
    required this.onChanged,
    super.key,
  });

  final LibraryGalleryQuery query;
  final ValueChanged<LibraryGalleryQuery> onChanged;

  @override
  Widget build(BuildContext context) => AmePopupMenuButton<_FilterChoice>(
    labels: const [
      LibraryDuplicateStrings.showDescendants,
      LibraryDuplicateStrings.hideDescendants,
      LibraryDuplicateStrings.allImages,
      LibraryDuplicateStrings.mergeDuplicates,
      "筛选条件…",
    ],
    hasContentIcon: false,
    leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
    items: [
      _choice(
        _FilterChoice.descendants,
        LibraryDuplicateStrings.showDescendants,
        query.includeDescendants,
        enabled: query.folderRelativePath != null,
      ),
      _choice(
        _FilterChoice.folderOnly,
        LibraryDuplicateStrings.hideDescendants,
        !query.includeDescendants,
        enabled: query.folderRelativePath != null,
      ),
      const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
      _choice(
        _FilterChoice.all,
        LibraryDuplicateStrings.allImages,
        query.duplicateMode == LibraryGalleryDuplicateMode.all,
      ),
      _choice(
        _FilterChoice.merged,
        LibraryDuplicateStrings.mergeDuplicates,
        query.duplicateMode == LibraryGalleryDuplicateMode.merged,
      ),
      const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
      _choice(_FilterChoice.metadata, "筛选条件…", query.metadataFilter.isActive),
    ],
    onSelected: (choice) async {
      if (choice == _FilterChoice.metadata) {
        final filter = await showLibraryMetadataFilterEditor(
          context: context,
          filter: query.metadataFilter,
        );
        if (context.mounted && filter != null) {
          onChanged(query.copyWith(metadataFilter: filter));
        }
        return;
      }
      onChanged(switch (choice) {
        _FilterChoice.descendants => query.copyWith(includeDescendants: true),
        _FilterChoice.folderOnly => query.copyWith(includeDescendants: false),
        _FilterChoice.all => query.copyWith(
          duplicateMode: LibraryGalleryDuplicateMode.all,
        ),
        _FilterChoice.merged => query.copyWith(
          duplicateMode: LibraryGalleryDuplicateMode.merged,
        ),
        _FilterChoice.metadata => query,
      });
    },
    builder: (context, openMenu) => AmeTooltip(
      message: query.metadataFilter.isActive
          ? "筛选（已设置条件）"
          : LibraryDuplicateStrings.filter,
      child: IconButton(
        key: const Key("library-filter-menu"),
        onPressed: openMenu,
        icon: const Icon(Symbols.filter_list_rounded),
      ),
    ),
  );

  PopupMenuEntry<_FilterChoice> _choice(
    _FilterChoice choice,
    String label,
    bool selected, {
    bool enabled = true,
  }) => ameMenuChoice(
    value: choice,
    isSelected: selected,
    enabled: enabled,
    label: label,
  );
}
