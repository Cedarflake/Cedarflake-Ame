import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_classification_review.dart";
import "../../application/library_classification_settings.dart";
import "../../application/library_organization_actions.dart";
import "../../application/library_operation_notice.dart";
import "../../application/library_file_operations.dart";
import "../../domain/library_file_operations.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_models.dart";
import "../coordination/library_organization_mode.dart";
import "library_album_viewer_action.dart";
import "library_favorite_button.dart";
import "library_organization_feedback.dart";
import "library_review_actions.dart";
import "library_review_mode_notice.dart";
import "library_viewer_classification.dart";
import "library_file_action_dialog.dart";
import "library_file_feedback.dart";

class LibraryViewerOrganizationControls extends ConsumerWidget {
  const LibraryViewerOrganizationControls({
    required this.target,
    required this.mode,
    required this.query,
    super.key,
  });
  final LibraryOrganizationTarget target;
  final LibraryOrganizationMode mode;
  final LibraryGalleryQuery query;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final entry = mode.current;
    final organization = ref.watch(
      libraryOrganizationInspectionProvider(target),
    );
    final categories = ref.watch(libraryClassificationSettingsProvider).value;
    final classification = categories?.visibleCategories.isNotEmpty == true
        ? ref.watch(libraryClassificationItemProvider(target))
        : null;
    final needsRetry =
        organization.hasError || classification?.hasError == true;
    final retrying =
        (organization.hasError && organization.isLoading) ||
        (classification?.hasError == true && classification?.isLoading == true);
    final retryLabel = retrying ? "正在读取图片状态" : "重新读取图片状态";
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        LibraryFavoriteButton(target: target, enabled: !mode.busy),
        if (!mode.isActive)
          LibraryAlbumViewerAction(target: target)
        else if (entry != null && entry.target == target)
          LibraryReviewActions(mode: mode, entry: entry),
        LibraryViewerClassification(target: target),
        AmeTooltip(
          message: "删除",
          child: IconButton(
            key: const Key("viewer-delete-file"),
            onPressed: mode.busy || ref.watch(libraryFileActionsProvider).busy
                ? null
                : () => showLibraryFileAction(
                    context: context,
                    ref: ref,
                    selection: LibraryFilesAtLocations([target]),
                    action: LibraryFileAction.recycle,
                  ),
            icon: const Icon(Symbols.delete_rounded, semanticLabel: "删除"),
          ),
        ),
        if (needsRetry)
          AmeTooltip(
            message: retryLabel,
            child: IconButton(
              key: const Key("viewer-reload-state"),
              onPressed: retrying || mode.busy
                  ? null
                  : () {
                      if (organization.hasError) {
                        ref.invalidate(
                          libraryOrganizationInspectionProvider(target),
                        );
                      }
                      if (classification?.hasError == true) {
                        ref.invalidate(
                          libraryClassificationItemProvider(target),
                        );
                      }
                    },
              icon: Icon(Symbols.refresh_rounded, semanticLabel: retryLabel),
            ),
          ),
      ],
    );
  }
}

class LibraryViewerOrganizationNotice extends ConsumerWidget {
  const LibraryViewerOrganizationNotice({required this.mode, super.key});
  final LibraryOrganizationMode mode;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final notice = ref.watch(libraryViewerOperationNoticeProvider);
    final files = ref.watch(libraryFileActionsProvider);
    if (files.progress != null) {
      return LibraryFileFeedback(activity: files, viewer: true);
    }
    final saving = ref.watch(libraryOrganizationActionsProvider).saving;
    if (mode.isActive && mode.problem != null) {
      return LibraryReviewModeNotice(mode: mode, enabled: !saving);
    }
    return notice == null
        ? const SizedBox.shrink()
        : LibraryOrganizationFeedback(notification: notice);
  }
}
