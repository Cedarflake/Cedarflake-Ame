import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_selection_checkbox.dart";
import "../../domain/library_gallery_group.dart";
import "library_duplicate_badge.dart";
import "library_gallery_card_metrics.dart";

class LibraryPhotoTileActions extends StatelessWidget {
  const LibraryPhotoTileActions({
    required this.locationId,
    required this.isSelectionVisible,
    required this.isSelected,
    required this.onToggleSelection,
    this.galleryGroup,
    super.key,
  });

  final String locationId;
  final LibraryGalleryGroup? galleryGroup;
  final bool isSelectionVisible;
  final bool isSelected;
  final VoidCallback onToggleSelection;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.all(LibraryGalleryCardMetrics.overlayInset),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Flexible(
            child: Padding(
              padding: const EdgeInsets.only(
                right: LibraryGalleryCardMetrics.actionGap,
              ),
              child: LibraryDuplicateBadge(
                locationId: locationId,
                galleryGroup: galleryGroup,
              ),
            ),
          ),
          // Reserve the selection target before hover so the badge does not reflow.
          SizedBox.square(
            dimension: LibraryGalleryCardMetrics.selectionExtent,
            child: isSelectionVisible
                ? AmeSelectionCheckbox(
                    key: ValueKey("select-$locationId"),
                    value: isSelected,
                    markExtent: LibraryGalleryCardMetrics.controlExtent,
                    tapTargetSize: LibraryGalleryCardMetrics.selectionExtent,
                    semanticLabel: "选择图片",
                    onChanged: (_) => onToggleSelection(),
                  )
                : null,
          ),
        ],
      ),
    );
  }
}
