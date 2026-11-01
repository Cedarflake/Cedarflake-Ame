import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_models.dart";
import "library_metadata_filter_editor.dart";

class LibrarySmartAlbumFilter extends StatelessWidget {
  const LibrarySmartAlbumFilter({
    required this.query,
    required this.onChanged,
    super.key,
  });

  final LibraryGalleryQuery query;
  final ValueChanged<LibraryGalleryQuery> onChanged;

  @override
  Widget build(BuildContext context) => AmePopupMenuButton<bool>(
    labels: const ["筛选条件…"],
    hasContentIcon: false,
    leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
    items: [
      ameMenuChoice(
        value: true,
        isSelected: query.metadataFilter.isActive,
        label: "筛选条件…",
      ),
    ],
    onSelected: (_) async {
      final filter = await showLibraryMetadataFilterEditor(
        context: context,
        filter: query.metadataFilter,
      );
      if (context.mounted && filter != null) {
        onChanged(query.copyWith(metadataFilter: filter));
      }
    },
    builder: (context, openMenu) => AmeTooltip(
      message: query.metadataFilter.isActive ? "筛选（已设置条件）" : "筛选",
      child: IconButton(
        key: const Key("library-filter-menu"),
        onPressed: openMenu,
        icon: const Icon(Symbols.filter_list_rounded),
      ),
    ),
  );
}
