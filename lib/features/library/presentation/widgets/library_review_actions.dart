import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_organization_actions.dart";
import "../../application/library_review_requests.dart";
import "../../application/library_review_session.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_review.dart";
import "../library_organization_mode.dart";
import "../library_organization_messages.dart";
import "../library_progress_controls.dart";
import "library_album_viewer_action.dart";
import "library_organization_menu.dart";

class LibraryReviewActions extends ConsumerStatefulWidget {
  const LibraryReviewActions({
    required this.mode,
    required this.entry,
    super.key,
  });
  final LibraryOrganizationMode mode;
  final LibraryReviewEntry entry;

  @override
  ConsumerState<LibraryReviewActions> createState() =>
      _LibraryReviewActionsState();
}

class _LibraryReviewActionsState extends ConsumerState<LibraryReviewActions> {
  LibraryReviewRequest? _request;
  LibraryProgressAction? _action;

  @override
  void didUpdateWidget(LibraryReviewActions oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.entry, widget.entry)) {
      _request?.cancel();
      _request = null;
    }
  }

  @override
  void dispose() {
    _request?.cancel();
    super.dispose();
  }

  Future<void> _save(LibraryProgressAction action) async {
    if (!mounted) return;
    final session = widget.mode.session;
    final entry = widget.entry;
    if (session == null || !session.owns(entry) || widget.mode.busy) return;
    if (_action != action || _request == null) {
      _request?.cancel();
      _action = action;
      _request = LibraryReviewRequest(
        session,
        entry,
        LibraryReviewProgress(action),
      );
    }
    final result = await ref
        .read(libraryOrganizationActionsProvider.notifier)
        .submit(_request!);
    if (!mounted || !identical(widget.entry, entry)) return;
    switch (result) {
      case LibraryOrganizationSaved():
        await widget.mode.afterSave(entry);
      case LibraryOrganizationRejected(:final failure):
        widget.mode.reportSaveFailure(
          entry,
          libraryOrganizationErrorMessage(failure.code),
          () => _save(action),
        );
      case LibraryOrganizationRetired():
        break;
    }
  }

  @override
  Widget build(BuildContext context) {
    final session = widget.mode.session;
    if (session == null) return const SizedBox.shrink();
    final busy =
        widget.mode.busy ||
        ref.watch(libraryOrganizationActionsProvider).saving;
    final entry = widget.entry;
    final target = entry.target;
    final inspection = ref.watch(libraryOrganizationInspectionProvider(target));
    final decision = inspection.isLoading || inspection.hasError
        ? null
        : inspection.value?.decisions.singleOrNull;
    final controls = decision == null
        ? null
        : LibraryProgressControls.forDisposition(
            decision.disposition,
            hasConflict: decision.hasConflict,
          );
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        LibraryAlbumViewerAction(
          target: target,
          enabled: !busy,
          membershipFactory: () => LibraryReviewAlbumMembership(session, entry),
          progressRequestFactory: (action) => LibraryReviewRequest(
            session,
            entry,
            LibraryReviewProgress(action),
          ),
          onSaved: () => unawaited(widget.mode.afterSave(entry)),
        ),
        LibraryOrganizationMenu(
          key: ValueKey(target),
          enabled: !busy,
          progress: controls?.ignore,
          defer: controls?.defer,
          onProgress: _save,
        ),
      ],
    );
  }
}
