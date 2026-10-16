import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../domain/library_duplicate_review.dart";
import "../../domain/library_models.dart";
import "../library_asset_summary.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_location_tile.dart";
import "library_path_text.dart";
import "library_viewer_filmstrip_thumbnail.dart";

class LibraryDuplicateGroupCard extends StatelessWidget {
  const LibraryDuplicateGroupCard({
    required this.entry,
    required this.onOpen,
    super.key,
  });
  final LibraryDuplicateReviewGroup entry;
  final VoidCallback onOpen;

  @override
  Widget build(BuildContext context) => _ReviewCard(
    asset: entry.representative,
    onOpen: onOpen,
    child: Column(
      mainAxisAlignment: MainAxisAlignment.center,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          displayLibraryFileName(entry.representative.displayPath),
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: Theme.of(context).textTheme.titleMedium,
        ),
        const SizedBox(height: 8),
        Text(
          [
            LibraryDuplicateStrings.locations(entry.group.locationCount),
            if (entry.group.physicalFileCount < entry.group.locationCount)
              LibraryDuplicateStrings.physicalFiles(
                entry.group.physicalFileCount,
              ),
            libraryFileSize(entry.representative.fileSize),
          ].join(" · "),
        ),
        TextButton(
          onPressed: onOpen,
          child: const Text(LibraryDuplicateStrings.viewLocations),
        ),
      ],
    ),
  );
}

class LibraryDuplicateMemberCard extends StatelessWidget {
  const LibraryDuplicateMemberCard({
    required this.member,
    required this.preferred,
    required this.onPrefer,
    required this.onOpen,
    super.key,
  });
  final LibraryDuplicateReviewLocation member;
  final bool preferred;
  final VoidCallback? onPrefer;
  final VoidCallback onOpen;

  @override
  Widget build(BuildContext context) => _ReviewCard(
    asset: member.asset,
    onOpen: onOpen,
    child: Column(
      mainAxisAlignment: MainAxisAlignment.center,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        LibraryDuplicateLocationTile(location: member.location),
        Wrap(
          spacing: 8,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            if (preferred) const Text(LibraryDuplicateStrings.preferred),
            TextButton.icon(
              key: ValueKey("prefer-${member.asset.locationId}"),
              onPressed: onPrefer,
              icon: Icon(
                preferred ? Symbols.star_rounded : Symbols.star_border_rounded,
              ),
              label: Text(
                preferred
                    ? LibraryDuplicateStrings.clearPreferred
                    : LibraryDuplicateStrings.prefer,
              ),
            ),
            TextButton(
              onPressed: onOpen,
              child: const Text(LibraryDuplicateStrings.openImage),
            ),
          ],
        ),
      ],
    ),
  );
}

class _ReviewCard extends StatelessWidget {
  const _ReviewCard({
    required this.asset,
    required this.onOpen,
    required this.child,
  });
  final LibraryAsset asset;
  final VoidCallback onOpen;
  final Widget child;

  @override
  Widget build(BuildContext context) => Card(
    child: Padding(
      padding: const EdgeInsets.all(12),
      child: LayoutBuilder(
        builder: (context, constraints) {
          final edge = (constraints.maxWidth / 4).clamp(64.0, 128.0);
          return Row(
            children: [
              SizedBox(
                width: edge,
                height: edge,
                child: LibraryViewerFilmstripThumbnail(
                  asset: asset,
                  isSelected: false,
                  onPressed: onOpen,
                  previewWidth: 144,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(child: child),
            ],
          );
        },
      ),
    ),
  );
}
