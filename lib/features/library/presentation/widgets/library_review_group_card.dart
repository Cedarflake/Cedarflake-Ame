import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_models.dart";
import "library_duplicate_review_metrics.dart";
import "library_path_text.dart";
import "library_viewer_filmstrip_thumbnail.dart";

/// Group previews and selection share one bounded image slot beside the details.
class LibraryReviewGroupCard extends StatelessWidget {
  const LibraryReviewGroupCard({
    required this.asset,
    required this.details,
    required this.actionLabel,
    required this.onOpen,
    this.thumbnail,
    this.selected = false,
    super.key,
  });

  final LibraryAsset asset;
  final Widget details;
  final String actionLabel;
  final VoidCallback onOpen;
  final Widget? thumbnail;
  final bool selected;

  @override
  Widget build(BuildContext context) => Card(
    margin: EdgeInsets.zero,
    clipBehavior: Clip.antiAlias,
    shape: RoundedRectangleBorder(
      borderRadius: BorderRadius.circular(12),
      side: BorderSide(
        color: selected
            ? Theme.of(context).colorScheme.primary
            : Colors.transparent,
        width: 1.5,
      ),
    ),
    child: InkWell(
      key: ValueKey("duplicate-card-${asset.locationId}"),
      onTap: onOpen,
      child: Padding(
        padding: const EdgeInsets.all(LibraryDuplicateReviewMetrics.padding),
        child: Row(
          children: [
            SizedBox.square(
              dimension: LibraryDuplicateReviewMetrics.groupPreviewExtent,
              child:
                  thumbnail ??
                  IgnorePointer(
                    child: ExcludeFocus(
                      child: ExcludeSemantics(
                        child: LibraryViewerFilmstripThumbnail(
                          asset: asset,
                          isSelected: false,
                          onPressed: onOpen,
                          previewWidth:
                              LibraryDuplicateReviewMetrics.groupPreviewWidth,
                        ),
                      ),
                    ),
                  ),
            ),
            const SizedBox(width: 12),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  AmeTooltip(
                    message: displayLibraryFileName(asset.displayPath),
                    child: Text(
                      displayLibraryFileName(asset.displayPath),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                  ),
                  const SizedBox(height: 4),
                  details,
                  const Spacer(),
                  Align(
                    alignment: Alignment.centerRight,
                    child: TextButton(
                      style: LibraryDuplicateReviewMetrics.actionStyle,
                      onPressed: onOpen,
                      child: Text(actionLabel),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    ),
  );
}
