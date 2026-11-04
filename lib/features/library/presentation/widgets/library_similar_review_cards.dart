import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../domain/library_exact_duplicates.dart";
import "../../domain/library_similar_review.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_review_cards.dart";
import "library_duplicate_review_metrics.dart";
import "library_duplicate_reveal_action.dart";

class LibrarySimilarGroupCard extends StatelessWidget {
  const LibrarySimilarGroupCard({
    required this.group,
    required this.onOpen,
    super.key,
  });
  final LibrarySimilarGroup group;
  final VoidCallback onOpen;

  @override
  Widget build(BuildContext context) => LibraryDuplicateReviewCard(
    asset: group.representative,
    onOpen: onOpen,
    details: Text(
      "${LibraryDuplicateStrings.similarGroups} · ${LibraryDuplicateStrings.similarImages(group.memberCount)}",
      maxLines: 2,
      overflow: TextOverflow.ellipsis,
    ),
    actions: [
      TextButton(
        style: LibraryDuplicateReviewMetrics.actionStyle,
        onPressed: onOpen,
        child: const Text(LibraryDuplicateStrings.viewSimilar),
      ),
    ],
  );
}

class LibrarySimilarMemberCard extends StatelessWidget {
  const LibrarySimilarMemberCard({
    required this.member,
    required this.preferred,
    required this.onPrefer,
    required this.onExclude,
    required this.onLocations,
    required this.onOpen,
    required this.selection,
    super.key,
  });
  final LibrarySimilarMember member;
  final bool preferred;
  final VoidCallback? onPrefer;
  final VoidCallback? onExclude;
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
        if (LibraryDuplicateStrings.availability(member.availability)
            case final label?)
          Text(label),
      ],
    ),
    actions: [
      AmeTooltip(
        message: preferred
            ? LibraryDuplicateStrings.clearPreferred
            : LibraryDuplicateStrings.prefer,
        child: IconButton(
          key: ValueKey("prefer-${member.asset.locationId}"),
          style: LibraryDuplicateReviewMetrics.iconActionStyle,
          onPressed: onPrefer,
          isSelected: preferred,
          icon: const Icon(Symbols.star_rounded),
          selectedIcon: const Icon(Symbols.star_rounded, fill: 1),
        ),
      ),
      AmeTooltip(
        message: LibraryDuplicateStrings.locations(member.locationCount),
        child: IconButton(
          key: ValueKey("similar-locations-${member.asset.locationId}"),
          style: LibraryDuplicateReviewMetrics.iconActionStyle,
          onPressed: onLocations,
          icon: const Icon(Symbols.stacks_rounded),
        ),
      ),
      LibraryDuplicateRevealAction(
        location: LibraryDuplicateLocation(
          locationId: member.asset.locationId,
          displayPath: member.asset.displayPath,
          availability: member.availability,
        ),
        builder: (context, reveal) => AmeTooltip(
          message: LibraryDuplicateStrings.revealLocation,
          child: IconButton(
            style: LibraryDuplicateReviewMetrics.iconActionStyle,
            onPressed: reveal,
            icon: const Icon(Symbols.folder_open_rounded),
          ),
        ),
      ),
      AmeTooltip(
        message: LibraryDuplicateStrings.exclude,
        child: IconButton(
          key: ValueKey("exclude-${member.asset.locationId}"),
          style: LibraryDuplicateReviewMetrics.iconActionStyle,
          onPressed: onExclude,
          icon: const Icon(Symbols.remove_rounded),
        ),
      ),
    ],
  );
}
