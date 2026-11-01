import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_menu_choice.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_models.dart";
import "library_metadata_filter_editor.dart";

enum _Choice { all, pending, confirmed, metadata }

class LibrarySmartAlbumFilter extends StatelessWidget {
  const LibrarySmartAlbumFilter({
    required this.query,
    required this.collection,
    required this.onChanged,
    super.key,
  });

  final LibraryGalleryQuery query;
  final LibrarySmartAlbumCollection collection;
  final ValueChanged<LibraryGalleryQuery> onChanged;

  @override
  Widget build(BuildContext context) => AmePopupMenuButton<_Choice>(
    labels: const ["全部结果", "待确认", "已确认", "筛选条件…"],
    hasContentIcon: false,
    leadingIconWidth: AmeMenuMetrics.selectionIndicatorSlotWidth,
    items: [
      for (final entry in const {
        _Choice.all: ("全部结果", LibraryClassificationReviewFilter.all),
        _Choice.pending: ("待确认", LibraryClassificationReviewFilter.pending),
        _Choice.confirmed: ("已确认", LibraryClassificationReviewFilter.confirmed),
      }.entries)
        ameMenuChoice(
          value: entry.key,
          isSelected: collection.review == entry.value.$2,
          label: entry.value.$1,
        ),
      const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
      ameMenuChoice(
        value: _Choice.metadata,
        isSelected: query.metadataFilter.isActive,
        label: "筛选条件…",
      ),
    ],
    onSelected: (choice) async {
      if (choice == _Choice.metadata) {
        final filter = await showLibraryMetadataFilterEditor(
          context: context,
          filter: query.metadataFilter,
        );
        if (context.mounted && filter != null) {
          onChanged(query.copyWith(metadataFilter: filter));
        }
        return;
      }
      final review = switch (choice) {
        _Choice.all => LibraryClassificationReviewFilter.all,
        _Choice.pending => LibraryClassificationReviewFilter.pending,
        _Choice.confirmed => LibraryClassificationReviewFilter.confirmed,
        _Choice.metadata => collection.review,
      };
      onChanged(
        query.copyWith(
          collection: LibraryGalleryCollection.smartAlbum(
            categoryId: collection.categoryId,
            review: review,
          ),
        ),
      );
    },
    builder: (context, openMenu) => AmeTooltip(
      message:
          query.metadataFilter.isActive ||
              collection.review != LibraryClassificationReviewFilter.all
          ? "筛选（已设置条件）"
          : "筛选",
      child: IconButton(
        key: const Key("library-filter-menu"),
        onPressed: openMenu,
        icon: const Icon(Symbols.filter_list_rounded),
      ),
    ),
  );
}
