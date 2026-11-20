import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_exact_duplicates.dart";
import "../../domain/library_similar_review.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_review_cards.dart";
import "library_duplicate_review_metrics.dart";
import "library_duplicate_reveal_action.dart";
import "library_review_group_card.dart";

class LibrarySimilarGroupCard extends StatelessWidget {
  const LibrarySimilarGroupCard({
    required this.group,
    required this.onOpen,
    this.thumbnail,
    this.selected = false,
    super.key,
  });
  final LibrarySimilarGroup group;
  final VoidCallback onOpen;
  final Widget? thumbnail;
  final bool selected;

  @override
  Widget build(BuildContext context) => LibraryReviewGroupCard(
    asset: group.representative,
    thumbnail: thumbnail,
    selected: selected,
    onOpen: onOpen,
    details: Text(
      "${context.l10n.similarGroups} · ${context.l10n.photoCount(group.memberCount)}",
      maxLines: 2,
      overflow: TextOverflow.ellipsis,
    ),
    actionLabel: context.l10n.viewSimilar,
  );
}

class LibrarySimilarMemberCard extends StatelessWidget {
  const LibrarySimilarMemberCard({
    required this.member,
    required this.preferred,
    required this.onPrefer,
    required this.onLocations,
    required this.onOpen,
    required this.selection,
    super.key,
  });
  final LibrarySimilarMember member;
  final bool preferred;
  final VoidCallback? onPrefer;
  final VoidCallback? onLocations;
  final VoidCallback onOpen;
  final Widget selection;

  @override
  Widget build(BuildContext context) => LibraryDuplicateReviewCard(
    asset: member.asset,
    onOpen: onOpen,
    inlineActions: true,
    selection: selection,
    details: Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          member.asset.displayPath,
          maxLines: 2,
          overflow: TextOverflow.ellipsis,
        ),
        if (LibraryDuplicateStrings.availability(
              context.l10n,
              member.availability,
            )
            case final label?)
          Text(label),
      ],
    ),
    actions: [
      AmeTooltip(
        message: preferred
            ? context.l10n.deselectPhoto
            : context.l10n.selectPhoto,
        child: IconButton(
          key: ValueKey("prefer-${member.asset.locationId}"),
          style: LibraryDuplicateReviewMetrics.iconActionStyle,
          onPressed: onPrefer,
          isSelected: preferred,
          icon: Icon(
            Symbols.bookmark_rounded,
            semanticLabel: context.l10n.selectPhoto,
          ),
          selectedIcon: Icon(
            Symbols.bookmark_rounded,
            fill: 1,
            semanticLabel: context.l10n.deselectPhoto,
          ),
        ),
      ),
      AmeTooltip(
        message: context.l10n.locationCount(member.locationCount),
        child: IconButton(
          key: ValueKey("similar-locations-${member.asset.locationId}"),
          style: LibraryDuplicateReviewMetrics.iconActionStyle,
          onPressed: onLocations,
          icon: const Icon(Symbols.photo_library_rounded),
        ),
      ),
      LibraryDuplicateRevealAction(
        location: LibraryDuplicateLocation(
          locationId: member.asset.locationId,
          displayPath: member.asset.displayPath,
          availability: member.availability,
        ),
        builder: (context, reveal) => AmeTooltip(
          message: context.l10n.openInExplorer,
          child: IconButton(
            style: LibraryDuplicateReviewMetrics.iconActionStyle,
            onPressed: reveal,
            icon: const Icon(Symbols.folder_open_rounded),
          ),
        ),
      ),
    ],
  );
}
