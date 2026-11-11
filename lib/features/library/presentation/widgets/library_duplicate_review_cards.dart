import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_duplicate_review.dart";
import "../../domain/library_exact_duplicates.dart";
import "../../domain/library_models.dart";
import "../../domain/library_organization.dart";
import "../library_asset_summary.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_reveal_action.dart";
import "library_duplicate_review_metrics.dart";
import "library_delete_file_button.dart";
import "library_path_text.dart";
import "library_viewer_filmstrip_thumbnail.dart";

class LibraryDuplicateGroupCard extends StatelessWidget {
  const LibraryDuplicateGroupCard({
    required this.entry,
    required this.onOpen,
    this.kindLabel,
    this.thumbnail,
    super.key,
  });
  final LibraryDuplicateReviewGroup entry;
  final VoidCallback onOpen;
  final String? kindLabel;
  final Widget? thumbnail;

  @override
  Widget build(BuildContext context) => LibraryDuplicateReviewCard(
    asset: entry.representative,
    thumbnail: thumbnail,
    onOpen: onOpen,
    details: Text(
      [
        ?kindLabel,
        LibraryDuplicateStrings.locations(entry.group.locationCount),
        if (entry.group.physicalFileCount < entry.group.locationCount)
          LibraryDuplicateStrings.physicalFiles(entry.group.physicalFileCount),
        libraryFileSize(entry.representative.fileSize),
      ].join(" · "),
      maxLines: 2,
      overflow: TextOverflow.ellipsis,
    ),
    actions: [
      TextButton(
        style: LibraryDuplicateReviewMetrics.actionStyle,
        onPressed: onOpen,
        child: const Text(LibraryDuplicateStrings.viewLocations),
      ),
    ],
  );
}

class LibraryDuplicateMemberCard extends StatelessWidget {
  const LibraryDuplicateMemberCard({
    required this.member,
    required this.preferred,
    required this.onPrefer,
    required this.onOpen,
    this.selection,
    super.key,
  });
  final LibraryDuplicateReviewLocation member;
  final bool preferred;
  final VoidCallback? onPrefer;
  final VoidCallback onOpen;
  final Widget? selection;

  @override
  Widget build(BuildContext context) {
    final path = member.location.displayPath.replaceAll("/", "\\");
    final separator = path.lastIndexOf("\\");
    final folder = separator < 0 ? "" : path.substring(0, separator);
    final availability = LibraryDuplicateStrings.availability(
      member.availability,
    );
    return LibraryDuplicateReviewCard(
      asset: member.asset,
      onOpen: onOpen,
      inlineActions: true,
      selection: selection,
      details: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: [
          if (folder.isNotEmpty)
            AmeTooltip(
              message: path,
              child: Text(folder, maxLines: 2, overflow: TextOverflow.ellipsis),
            ),
          if (availability != null) Text(availability),
        ],
      ),
      actions: [
        AmeTooltip(
          message: preferred
              ? LibraryDuplicateStrings.clearPreferred
              : LibraryDuplicateStrings.prefer,
          child: IconButton(
            style: LibraryDuplicateReviewMetrics.iconActionStyle,
            key: ValueKey("prefer-${member.asset.locationId}"),
            onPressed: onPrefer,
            isSelected: preferred,
            icon: const Icon(
              Symbols.star_rounded,
              semanticLabel: LibraryDuplicateStrings.prefer,
            ),
            selectedIcon: const Icon(
              Symbols.star_rounded,
              fill: 1,
              semanticLabel: LibraryDuplicateStrings.clearPreferred,
            ),
          ),
        ),
        LibraryDuplicateRevealAction(
          location: member.location,
          builder: (context, reveal) => AmeTooltip(
            message: LibraryDuplicateStrings.revealLocation,
            child: IconButton(
              style: LibraryDuplicateReviewMetrics.iconActionStyle,
              key: ValueKey("reveal-${member.asset.locationId}"),
              onPressed: reveal,
              icon: const Icon(
                Symbols.folder_open_rounded,
                semanticLabel: LibraryDuplicateStrings.revealLocation,
              ),
            ),
          ),
        ),
        LibraryDeleteFileButton(
          target: LibraryOrganizationTarget.fromAsset(member.asset),
          enabled: member.availability == LibraryDuplicateAvailability.local,
          style: LibraryDuplicateReviewMetrics.iconActionStyle,
          buttonKey: ValueKey("delete-file-${member.asset.locationId}"),
        ),
      ],
    );
  }
}

/// Groups and members share content and primary interaction with distinct actions.
class LibraryDuplicateReviewCard extends StatelessWidget {
  const LibraryDuplicateReviewCard({
    required this.asset,
    required this.onOpen,
    required this.details,
    required this.actions,
    this.inlineActions = false,
    this.selection,
    this.thumbnail,
    super.key,
  });
  final LibraryAsset asset;
  final VoidCallback onOpen;
  final Widget details;
  final List<Widget> actions;
  final bool inlineActions;
  final Widget? selection;
  final Widget? thumbnail;

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final inline =
          inlineActions &&
          LibraryDuplicateReviewMetrics.inlineMemberActions(
            context,
            constraints.maxWidth,
            actions: actions.length,
            selectable: selection != null,
          );
      final content = Row(
        children: [
          if (selection != null)
            SizedBox(
              width: LibraryDuplicateReviewMetrics.selectionExtent,
              child: selection,
            ),
          thumbnail ??
              ExcludeFocus(
                child: ExcludeSemantics(
                  child: IgnorePointer(
                    child: SizedBox.square(
                      dimension: LibraryDuplicateReviewMetrics.thumbnailExtent,
                      child: LibraryViewerFilmstripThumbnail(
                        asset: asset,
                        isSelected: false,
                        onPressed: onOpen,
                        previewWidth: 144,
                      ),
                    ),
                  ),
                ),
              ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
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
              ],
            ),
          ),
          if (inline) ...[
            const SizedBox(width: 12),
            Row(mainAxisSize: MainAxisSize.min, children: actions),
          ],
        ],
      );
      return Card(
        margin: EdgeInsets.zero,
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          key: ValueKey("duplicate-card-${asset.locationId}"),
          onTap: onOpen,
          child: Padding(
            padding: const EdgeInsets.all(
              LibraryDuplicateReviewMetrics.padding,
            ),
            child: inline
                ? content
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Expanded(child: content),
                      const SizedBox(height: LibraryDuplicateReviewMetrics.gap),
                      Wrap(
                        alignment: WrapAlignment.end,
                        spacing: 8,
                        runSpacing: 4,
                        children: actions,
                      ),
                    ],
                  ),
          ),
        ),
      );
    },
  );
}
