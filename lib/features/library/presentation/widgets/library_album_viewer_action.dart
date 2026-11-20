import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/messages/ame_message.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_message_text.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../settings/application/ame_preferences.dart";
import "../../application/library_album_proposal.dart";
import "../../application/library_album_requests.dart";
import "../../application/library_organization.dart";
import "../../application/library_organization_actions.dart";
import "../../application/library_organization_requests.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_review.dart";
import "../library_organization_messages.dart";
import "../library_progress_controls.dart";
import "library_album_picker.dart";

class LibraryAlbumViewerAction extends ConsumerStatefulWidget {
  const LibraryAlbumViewerAction({
    required this.target,
    this.membershipFactory,
    this.progressRequestFactory,
    this.onSaved,
    this.enabled = true,
    super.key,
  });
  final LibraryOrganizationTarget target;
  final LibraryAlbumMembership Function()? membershipFactory;
  final LibraryOrganizationRequest Function(LibraryProgressAction)?
  progressRequestFactory;
  final VoidCallback? onSaved;
  final bool enabled;

  @override
  ConsumerState<LibraryAlbumViewerAction> createState() =>
      _LibraryAlbumViewerActionState();
}

class _LibraryAlbumViewerActionState
    extends ConsumerState<LibraryAlbumViewerAction> {
  Object? _opening;
  LibraryAlbumMembership? _membership;
  AmeMessage? _error;

  @override
  void didUpdateWidget(LibraryAlbumViewerAction oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.target != oldWidget.target) {
      _membership?.dispose();
      _opening = null;
      _error = null;
    }
  }

  @override
  void dispose() {
    _membership?.dispose();
    _opening = null;
    super.dispose();
  }

  Future<void> _open() async {
    if (_opening != null || !widget.enabled) return;
    final target = widget.target;
    final opening = Object();
    setState(() {
      _opening = opening;
      _error = null;
    });
    bool owns() =>
        mounted && identical(_opening, opening) && widget.target == target;
    LibraryAlbumMembership? membership;
    try {
      final preparation = await prepareLibraryAlbumProposal(
        service: ref.read(libraryOrganizationProvider),
        target: target,
        preferences: ref.read(amePreferencesControllerProvider).albums,
      );
      if (!owns()) return;
      final proposal = preparation.proposal;
      membership =
          widget.membershipFactory?.call() ??
          LibraryAlbumMembershipSession([
            target,
          ], isMove: proposal.current != null);
      _membership = membership;
      AmeMessage? initialError;
      if (!proposal.shouldAsk && proposal.destination != null) {
        final result = await ref
            .read(libraryOrganizationActionsProvider.notifier)
            .submit(membership.request(proposal.destination));
        if (!owns()) return;
        if (result is LibraryOrganizationSaved) {
          widget.onSaved?.call();
          return;
        }
        if (result case LibraryOrganizationRejected(:final failure)) {
          initialError = libraryOrganizationErrorMessage(failure.code);
        }
      }
      if (!mounted || !owns()) return;
      final choice = await showLibraryAlbumPicker(
        context: context,
        actionLabel: proposal.current == null
            ? context.l10n.addToAlbum
            : context.l10n.moveToAnotherAlbum,
        actionIcon: proposal.current == null
            ? Symbols.bookmark_add_rounded
            : Symbols.drive_file_move_rounded,
        initialAlbum: proposal.destination,
        currentAlbum: proposal.current,
        clearLabel: proposal.current == null
            ? null
            : context.l10n.removeFromAlbum,
        explanation: proposal.hasConflict
            ? context.l10n.albumGroupConflict
            : null,
        conflictTarget: proposal.hasConflict ? target : null,
        initialError: initialError,
        membership: membership,
        progressRequestFactory: widget.progressRequestFactory,
        progressControls: LibraryProgressControls.forDisposition(
          context.l10n,
          preparation.decision.disposition,
          hasConflict: preparation.decision.hasConflict,
        ),
      );
      if (choice != null && owns()) widget.onSaved?.call();
    } on Object catch (error) {
      if (owns()) {
        setState(
          () => _error = error is LibraryOrganizationFailure
              ? libraryOrganizationErrorMessage(error.code)
              : AmeLabel.albumDetailsUnavailable,
        );
      }
    } finally {
      membership?.dispose();
      if (owns()) setState(() => _opening = null);
    }
  }

  @override
  Widget build(BuildContext context) {
    final busy = ref.watch(
      libraryOrganizationActionsProvider.select((state) => state.saving),
    );
    final decision = ref
        .watch(libraryOrganizationInspectionProvider(widget.target))
        .value
        ?.decisions
        .singleOrNull;
    final hasConflict = decision?.hasConflict == true;
    final label =
        _error?.resolve(context.l10n) ??
        switch (decision) {
          LibraryOrganizationDecision(hasConflict: true) =>
            context.l10n.albumConflict,
          LibraryOrganizationDecision(album: LibraryOrganizationAlbum()) =>
            context.l10n.moveToAnotherAlbum,
          _ => context.l10n.addToAlbum,
        };
    final onPressed = busy || _opening != null || !widget.enabled
        ? null
        : _open;
    return AmeTooltip(
      message: label,
      child: IconButton(
        key: Key(hasConflict ? "viewer-album-conflict" : "viewer-album-action"),
        onPressed: onPressed,
        icon: Icon(
          _error != null || hasConflict
              ? Symbols.error_rounded
              : Symbols.photo_album_rounded,
          size: 20,
          semanticLabel: label,
        ),
      ),
    );
  }
}
