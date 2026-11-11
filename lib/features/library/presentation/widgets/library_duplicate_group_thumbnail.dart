import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_flip_transition.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_models.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_review_metrics.dart";
import "library_viewer_filmstrip_thumbnail.dart";

class LibraryDuplicateGroupThumbnail extends StatelessWidget {
  const LibraryDuplicateGroupThumbnail({
    required this.asset,
    required this.selected,
    required this.onPressed,
    super.key,
  });

  final LibraryAsset asset;
  final bool selected;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final label = LibraryDuplicateStrings.selectGroup(
      asset.relativePath,
      selected: selected,
    );
    const dimension = LibraryDuplicateReviewMetrics.thumbnailExtent;
    return Semantics(
      label: label,
      child: AmeTooltip(
        message: label,
        child: IconButton(
          onPressed: onPressed,
          isSelected: selected,
          style: IconButton.styleFrom(
            fixedSize: const Size.square(dimension),
            padding: EdgeInsets.zero,
            shape: RoundedRectangleBorder(
              borderRadius: BorderRadius.circular(8),
            ),
          ),
          icon: SizedBox.square(
            dimension: dimension,
            child: AmeFlipTransition(
              showBack: selected,
              front: LibraryViewerFilmstripThumbnail(
                asset: asset,
                isSelected: false,
                onPressed: onPressed,
                previewWidth: 144,
              ),
              back: DecoratedBox(
                decoration: BoxDecoration(
                  color: colors.primaryContainer,
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Icon(
                  Symbols.check_rounded,
                  size: 32,
                  color: colors.onPrimaryContainer,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
