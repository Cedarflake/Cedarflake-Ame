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

enum _OrganizationMenuAction { undo, redo, progress }

class LibraryOrganizationMenu extends ConsumerStatefulWidget {
  const LibraryOrganizationMenu({
    required this.enabled,
    this.progress,
    this.onProgress,
    super.key,
  });

  final bool enabled;
  final LibraryProgressControl? progress;
  final ValueChanged<LibraryProgressAction>? onProgress;

  @override
  ConsumerState<LibraryOrganizationMenu> createState() =>
      _LibraryOrganizationMenuState();
}

class _LibraryOrganizationMenuState
    extends ConsumerState<LibraryOrganizationMenu> {
  ({LibraryOrganizationHistory? history, LibraryProgressControl? progress})?
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
      onOpened: () => _opened = (history: history, progress: widget.progress),
      onOpenChanged: (open) {
        if (!open) _opened = null;
      },
      labels: [
        "撤销操作",
        "重做操作",
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
        if (widget.progress case final progress?) ...[
          const PopupMenuDivider(),
          PopupMenuItem(
            value: _OrganizationMenuAction.progress,
            enabled: canAct && progress.enabled && widget.onProgress != null,
            child: AmeMenuItemContent(
              label: progress.label,
              icon: progress.icon,
            ),
          ),
        ],
      ],
      onSelected: (action) {
        switch (action) {
          case _OrganizationMenuAction.undo:
            step(LibraryOrganizationHistoryDirection.undo);
          case _OrganizationMenuAction.redo:
            step(LibraryOrganizationHistoryDirection.redo);
          case _OrganizationMenuAction.progress:
            if (_opened?.progress case final progress?) {
              if (canAct &&
                  progress.enabled &&
                  widget.progress?.enabled == true &&
                  widget.progress?.action == progress.action) {
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
