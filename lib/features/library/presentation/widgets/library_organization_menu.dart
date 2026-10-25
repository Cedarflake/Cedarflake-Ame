import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../application/library_organization_actions.dart";
import "../../application/library_organization_history.dart";
import "../../domain/library_organization_history.dart";
import "../../domain/library_review.dart";
import "../library_progress_controls.dart";

enum _OrganizationMenuAction { undo, redo, defer, progress }

class LibraryOrganizationMenu extends ConsumerStatefulWidget {
  const LibraryOrganizationMenu({
    required this.enabled,
    this.progress,
    this.defer,
    this.onProgress,
    super.key,
  });

  final bool enabled;
  final LibraryProgressControl? progress;
  final LibraryProgressControl? defer;
  final ValueChanged<LibraryProgressAction>? onProgress;

  @override
  ConsumerState<LibraryOrganizationMenu> createState() =>
      _LibraryOrganizationMenuState();
}

class _LibraryOrganizationMenuState
    extends ConsumerState<LibraryOrganizationMenu> {
  ({
    LibraryOrganizationHistory? history,
    LibraryProgressControl? progress,
    LibraryProgressControl? defer,
  })?
  _opened;

  @override
  Widget build(BuildContext context) {
    final result = ref.watch(libraryOrganizationHistoryProvider);
    final saving = ref.watch(libraryOrganizationActionsProvider).saving;
    final history = result.isLoading || result.hasError ? null : result.value;
    final canAct = widget.enabled && !saving;
    void step(LibraryOrganizationHistoryDirection direction) {
      final observed = _opened?.history;
      if (!canAct || observed == null) return;
      final originalId = observed.operationFor(direction);
      if (originalId == null) return;
      unawaited(
        ref
            .read(libraryOrganizationActionsProvider.notifier)
            .submit(
              LibraryOrganizationHistoryRequest(
                direction: direction,
                originalId: originalId,
                revision: observed.revision,
              ),
            ),
      );
    }

    return AmePopupMenuButton<_OrganizationMenuAction>(
      onOpened: () => _opened = (
        history: history,
        progress: widget.progress,
        defer: widget.defer,
      ),
      onOpenChanged: (open) {
        if (!open) _opened = null;
      },
      labels: [
        "撤销操作",
        "重做操作",
        if (widget.defer case final defer?) defer.label,
        if (widget.progress case final progress?) progress.label,
      ],
      items: [
        PopupMenuItem(
          value: _OrganizationMenuAction.undo,
          enabled: canAct && history?.undoOperationId != null,
          child: const AmeMenuItemContent(
            label: "撤销操作",
            icon: Symbols.undo_rounded,
          ),
        ),
        PopupMenuItem(
          value: _OrganizationMenuAction.redo,
          enabled: canAct && history?.redoOperationId != null,
          child: const AmeMenuItemContent(
            label: "重做操作",
            icon: Symbols.redo_rounded,
          ),
        ),
        if (widget.defer != null || widget.progress != null)
          const PopupMenuDivider(),
        if (widget.defer case final defer?)
          PopupMenuItem(
            key: const Key("library-review-menu-defer"),
            value: _OrganizationMenuAction.defer,
            enabled: canAct && defer.enabled && widget.onProgress != null,
            child: AmeMenuItemContent(label: defer.label, icon: defer.icon),
          ),
        if (widget.progress case final progress?)
          PopupMenuItem(
            value: _OrganizationMenuAction.progress,
            enabled: canAct && progress.enabled && widget.onProgress != null,
            child: AmeMenuItemContent(
              label: progress.label,
              icon: progress.icon,
            ),
          ),
      ],
      onSelected: (action) {
        switch (action) {
          case _OrganizationMenuAction.undo:
            step(LibraryOrganizationHistoryDirection.undo);
          case _OrganizationMenuAction.redo:
            step(LibraryOrganizationHistoryDirection.redo);
          case _OrganizationMenuAction.defer:
          case _OrganizationMenuAction.progress:
            final observed = action == _OrganizationMenuAction.defer
                ? _opened?.defer
                : _opened?.progress;
            final current = action == _OrganizationMenuAction.defer
                ? widget.defer
                : widget.progress;
            if (observed case final progress?) {
              if (canAct &&
                  progress.enabled &&
                  current?.enabled == true &&
                  current?.action == progress.action) {
                widget.onProgress?.call(progress.action);
              }
            }
        }
      },
      builder: (context, openMenu) => IconButton(
        key: const Key("library-review-actions-menu"),
        tooltip: result.hasError ? "操作记录未能加载，点击重试" : "整理选项",
        onPressed: !canAct
            ? null
            : () {
                if (result.hasError) {
                  ref.invalidate(libraryOrganizationHistoryProvider);
                } else {
                  openMenu();
                }
              },
        icon: const Icon(Symbols.more_horiz_rounded, size: 20),
      ),
    );
  }
}
