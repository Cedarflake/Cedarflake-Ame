import "dart:async";

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_models.dart";
import "../library_duplicate_strings.dart";
import "../library_filter_strings.dart";
import "library_metadata_filter_dialog.dart";

enum _FilterChoice { descendants, folderOnly, all, merged, metadata }

class LibraryGalleryFilter extends StatefulWidget {
  const LibraryGalleryFilter({
    required this.query,
    required this.onChanged,
    super.key,
  });

  final LibraryGalleryQuery query;
  final ValueChanged<LibraryGalleryQuery> onChanged;

  @override
  State<LibraryGalleryFilter> createState() => _LibraryGalleryFilterState();
}

class _LibraryGalleryFilterState extends State<LibraryGalleryFilter> {
  LibraryGalleryQuery get query => widget.query;

  Future<void> _editMetadata() async {
    final openingQuery = query;
    final filter = await showLibraryMetadataFilterDialog(
      context,
      query.metadataFilter,
    );
    if (!mounted || filter == null || query != openingQuery) return;
    if (filter != query.metadataFilter) {
      widget.onChanged(query.copyWith(metadataFilter: filter));
    }
  }

  @override
  Widget build(BuildContext context) {
    return AmePopupMenuButton<_FilterChoice>(
      labels: const [
        LibraryDuplicateStrings.showDescendants,
        LibraryDuplicateStrings.hideDescendants,
        LibraryDuplicateStrings.allImages,
        LibraryDuplicateStrings.mergeDuplicates,
        LibraryFilterStrings.more,
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
          !query.foldsDuplicates,
          enabled: !query.requiresMergedDuplicates,
        ),
        _choice(
          _FilterChoice.merged,
          LibraryDuplicateStrings.mergeDuplicates,
          query.foldsDuplicates,
          enabled: !query.requiresMergedDuplicates,
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        const PopupMenuItem(
          value: _FilterChoice.metadata,
          child: Padding(
            padding: EdgeInsets.only(
              left:
                  AmeMenuMetrics.selectionIndicatorSlotWidth +
                  AmeMenuMetrics.iconLabelGap,
            ),
            child: Text(LibraryFilterStrings.more),
          ),
        ),
      ],
      onSelected: (choice) {
        if (choice == _FilterChoice.metadata) {
          unawaited(_editMetadata());
          return;
        }
        widget.onChanged(switch (choice) {
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
        message: LibraryDuplicateStrings.filter,
        child: IconButton(
          key: const Key("library-filter-menu"),
          onPressed: openMenu,
          icon: const Icon(Symbols.filter_list_rounded),
        ),
      ),
    );
  }

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
