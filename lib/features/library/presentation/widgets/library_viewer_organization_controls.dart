import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../application/library_organization_actions.dart";
import "../../domain/library_organization.dart";
import "../library_organization_mode.dart";
import "library_album_viewer_action.dart";
import "library_favorite_button.dart";
import "library_organization_feedback.dart";
import "library_review_actions.dart";
import "library_viewer_notice.dart";

class LibraryViewerOrganizationControls extends StatelessWidget {
  const LibraryViewerOrganizationControls({
    required this.target,
    required this.mode,
    super.key,
  });
  final LibraryOrganizationTarget target;
  final LibraryOrganizationMode mode;

  @override
  Widget build(BuildContext context) {
    final entry = mode.current;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        LibraryFavoriteButton(target: target, enabled: !mode.busy),
        if (!mode.isActive)
          LibraryAlbumViewerAction(target: target)
        else if (entry != null && entry.target == target)
          LibraryReviewActions(mode: mode, entry: entry),
      ],
    );
  }
}

class LibraryViewerOrganizationNotice extends ConsumerWidget {
  const LibraryViewerOrganizationNotice({required this.mode, super.key});
  final LibraryOrganizationMode mode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final notice = ref.watch(libraryOrganizationNoticeProvider);
    final saving = ref.watch(libraryOrganizationActionsProvider).saving;
    if (mode.isActive && mode.problem != null) {
      return LibraryViewerNotice(
        icon: Symbols.error_rounded,
        message: mode.problem!,
        actionLabel: "重试",
        onAction: mode.busy || saving ? null : mode.retry,
      );
    }
    return notice == null
        ? const SizedBox.shrink()
        : LibraryOrganizationFeedback(notification: notice);
  }
}
