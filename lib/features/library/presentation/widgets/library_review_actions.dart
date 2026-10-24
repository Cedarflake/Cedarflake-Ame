import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../application/library_organization_actions.dart";
import "../../application/library_review_requests.dart";
import "../../application/library_review_session.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_review.dart";
import "../library_organization_mode.dart";
import "../library_organization_messages.dart";
import "library_album_viewer_action.dart";

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
    final target = widget.entry.target;
    final decision = ref
        .watch(libraryOrganizationInspectionProvider(target))
        .value
        ?.decisions
        .singleOrNull;
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        LibraryAlbumViewerAction(
          target: target,
          enabled: !busy,
          membershipFactory: () =>
              LibraryReviewAlbumMembership(session, widget.entry),
          onSaved: () => unawaited(widget.mode.afterSave(widget.entry)),
        ),
        TextButton.icon(
          key: const Key("viewer-review-defer"),
          onPressed: busy ? null : () => _save(LibraryProgressAction.defer),
          icon: const Icon(Symbols.schedule_rounded, size: 20),
          label: const Text("稍后整理"),
        ),
        LibraryProgressMenu(
          enabled: !busy,
          ignored:
              decision?.disposition == LibraryOrganizationDisposition.ignored,
          onSelected: _save,
        ),
      ],
    );
  }
}

class LibraryProgressMenu extends StatelessWidget {
  const LibraryProgressMenu({
    required this.enabled,
    required this.ignored,
    required this.onSelected,
    super.key,
  });
  final bool enabled;
  final bool ignored;
  final ValueChanged<LibraryProgressAction> onSelected;

  @override
  Widget build(BuildContext context) {
    final label = ignored ? "恢复待整理" : "忽略";
    final action = ignored
        ? LibraryProgressAction.restore
        : LibraryProgressAction.ignore;
    return AmePopupMenuButton<LibraryProgressAction>(
      labels: [label],
      items: [
        PopupMenuItem(
          value: action,
          child: AmeMenuItemContent(
            label: label,
            icon: ignored ? Symbols.undo_rounded : Symbols.hide_image_rounded,
          ),
        ),
      ],
      onSelected: (action) {
        if (enabled) onSelected(action);
      },
      builder: (context, openMenu) => IconButton(
        key: const Key("library-review-actions-menu"),
        tooltip: "整理选项",
        onPressed: enabled ? openMenu : null,
        icon: const Icon(Symbols.more_horiz_rounded),
      ),
    );
  }
}
