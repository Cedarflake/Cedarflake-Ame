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
    return Semantics(
      label: label,
      button: true,
      selected: selected,
      child: AmeTooltip(
        message: label,
        child: InkWell(
          onTap: onPressed,
          borderRadius: BorderRadius.circular(8),
          child: SizedBox(
            height: LibraryDuplicateReviewMetrics.groupPreviewHeight,
            width: double.infinity,
            child: AmeFlipTransition(
              showBack: selected,
              front: IgnorePointer(
                child: ExcludeFocus(
                  child: ExcludeSemantics(
                    child: LibraryViewerFilmstripThumbnail(
                      asset: asset,
                      isSelected: false,
                      onPressed: onPressed,
                      previewWidth:
                          LibraryDuplicateReviewMetrics.groupPreviewWidth,
                    ),
                  ),
                ),
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
