import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_models.dart";
import "../library_duplicate_strings.dart";

enum _FilterChoice { descendants, folderOnly, all, merged }

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
    ],
    onSelected: (choice) => onChanged(switch (choice) {
      _FilterChoice.descendants => query.copyWith(includeDescendants: true),
      _FilterChoice.folderOnly => query.copyWith(includeDescendants: false),
      _FilterChoice.all => query.copyWith(
        duplicateMode: LibraryGalleryDuplicateMode.all,
      ),
      _FilterChoice.merged => query.copyWith(
        duplicateMode: LibraryGalleryDuplicateMode.merged,
      ),
    }),
    builder: (context, openMenu) => AmeTooltip(
      message: LibraryDuplicateStrings.filter,
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
