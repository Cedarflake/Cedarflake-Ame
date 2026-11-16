import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_motion.dart";
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
            child: Stack(
              fit: StackFit.expand,
              children: [
                IgnorePointer(
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
                Positioned(
                  top: 8,
                  left: 8,
                  child: AmeStateTransition(
                    active: selected,
                    motion: AmeMotion.fastEffects,
                    builder: (context, opacity, child) =>
                        FadeTransition(opacity: opacity, child: child),
                    child: DecoratedBox(
                      decoration: BoxDecoration(
                        color: colors.primary,
                        shape: BoxShape.circle,
                      ),
                      child: SizedBox.square(
                        dimension: 24,
                        child: Icon(
                          Symbols.check_rounded,
                          size: 18,
                          color: colors.onPrimary,
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
