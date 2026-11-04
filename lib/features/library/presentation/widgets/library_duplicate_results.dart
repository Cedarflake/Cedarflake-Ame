import "dart:async";

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../domain/library_exact_duplicates.dart";
import "../../domain/library_models.dart";
import "../../domain/library_similar_review.dart";
import "../coordination/library_duplicate_workspace.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_review_cards.dart";
import "library_duplicate_review_grid.dart";
import "library_gallery_message.dart";
import "library_similar_review_cards.dart";

class LibraryDuplicateResults extends StatelessWidget {
  const LibraryDuplicateResults({
    required this.workspace,
    required this.analyzing,
    required this.onOpen,
    required this.onPreferExact,
    required this.onLocations,
    super.key,
  });
  final LibraryDuplicateWorkspace workspace;
  final bool analyzing;
  final void Function(LibraryAsset asset, int index) onOpen;
  final ValueChanged<LibraryAsset?> onPreferExact;
  final ValueChanged<LibrarySimilarMember> onLocations;

  @override
  Widget build(BuildContext context) {
    if (workspace.itemCount == 0) {
      if (workspace.loading) return const SizedBox.shrink();
      final message = workspace.hasMembers
          ? LibraryDuplicateStrings.emptyLocations
          : switch (workspace.view) {
              LibraryDuplicateView.ignored =>
                LibraryDuplicateStrings.noIgnoredGroups,
              LibraryDuplicateView.exact =>
                analyzing
                    ? LibraryDuplicateStrings.waitingForResults
                    : LibraryDuplicateStrings.emptyAfterAnalysis,
              LibraryDuplicateView.similar =>
                analyzing
                    ? LibraryDuplicateStrings.waitingForSimilar
                    : LibraryDuplicateStrings.emptySimilar,
            };
      return LibraryGalleryMessage(
        icon: Symbols.photo_library_rounded,
        title: message,
      );
    }
    return LibraryDuplicateReviewGrid(
      key: ValueKey(workspace.pageKey),
      itemCount: workspace.itemCount,
      singleColumn: workspace.hasMembers,
      selectable: workspace.hasMembers,
      memberActions: workspace.isSimilar ? 3 : 2,
      onVisibleRange: workspace.ensureRange,
      assetAt: workspace.assetAt,
      itemBuilder: (context, index) {
        if (!workspace.hasMembers) {
          final entry = workspace.groupAt(index);
          return switch (entry) {
            LibraryExactGroupEntry(:final group) => LibraryDuplicateGroupCard(
              entry: group,
              kindLabel: workspace.view == LibraryDuplicateView.ignored
                  ? LibraryDuplicateStrings.pendingGroups
                  : null,
              onOpen: () => unawaited(workspace.openGroup(entry)),
            ),
            LibrarySimilarGroupEntry(:final group) => LibrarySimilarGroupCard(
              key: ValueKey("similar-group-${group.id}"),
              group: group,
              onOpen: () => unawaited(workspace.openGroup(entry)),
            ),
            null => const Card(margin: EdgeInsets.zero),
          };
        }
        if (workspace.isSimilar) {
          final member = workspace.similar.memberWindow.itemAt(index);
          if (member == null) return const Card(margin: EdgeInsets.zero);
          final preferred =
              workspace.similar.decision.preferred == member.content;
          final canWrite = !workspace.saving && !workspace.loading;
          return LibrarySimilarMemberCard(
            member: member,
            preferred: preferred,
            selection: _selection(member.asset),
            onOpen: () => onOpen(member.asset, index),
            onPrefer:
                canWrite &&
                    (preferred ||
                        member.availability ==
                            LibraryDuplicateAvailability.local)
                ? () => unawaited(
                    workspace.similar.prefer(preferred ? null : member),
                  )
                : null,
            onLocations: member.locationCount > 1
                ? () => onLocations(member)
                : null,
          );
        }
        final member = workspace.exact.memberWindow.itemAt(index);
        if (member == null) return const Card(margin: EdgeInsets.zero);
        final preferred = workspace.exact.decision.prefers(member.asset);
        return LibraryDuplicateMemberCard(
          member: member,
          preferred: preferred,
          selection: _selection(member.asset),
          onOpen: () => onOpen(member.asset, index),
          onPrefer:
              !workspace.saving &&
                  !workspace.loading &&
                  (preferred ||
                      member.availability == LibraryDuplicateAvailability.local)
              ? () => onPreferExact(preferred ? null : member.asset)
              : null,
        );
      },
    );
  }

  Widget _selection(LibraryAsset asset) => Checkbox(
    key: ValueKey("compare-select-${asset.locationId}"),
    semanticLabel:
        "${LibraryDuplicateStrings.selectComparison}：${asset.relativePath}",
    value: workspace.isCompared(asset),
    onChanged: workspace.canCompare(asset)
        ? (_) => workspace.toggleComparison(asset)
        : null,
  );
}
