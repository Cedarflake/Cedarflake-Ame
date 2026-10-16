import "package:flutter/material.dart";

import "library_duplicate_badge.dart";
import "library_gallery_card_metrics.dart";

class LibraryPhotoTileActions extends StatelessWidget {
  const LibraryPhotoTileActions({
    required this.locationId,
    required this.isSelectionVisible,
    required this.isSelected,
    required this.onToggleSelection,
    super.key,
  });

  final String locationId;
  final bool isSelectionVisible;
  final bool isSelected;
  final VoidCallback onToggleSelection;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.all(LibraryGalleryCardMetrics.overlayInset),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Flexible(child: LibraryDuplicateBadge(locationId: locationId)),
          const SizedBox(width: LibraryGalleryCardMetrics.actionGap),
          // Reserve the selection target before hover so the badge does not reflow.
          SizedBox(
            width: LibraryGalleryCardMetrics.selectionExtent,
            child: isSelectionVisible
                ? Material(
                    color: colorScheme.surface.withValues(alpha: 0.92),
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: Checkbox(
                      key: ValueKey("select-$locationId"),
                      value: isSelected,
                      onChanged: (_) => onToggleSelection(),
                      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      visualDensity: VisualDensity.compact,
                    ),
                  )
                : null,
          ),
        ],
      ),
    );
  }
}
