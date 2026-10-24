import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../application/library_organization_actions.dart";
import "../../application/library_organization_history.dart";
import "../../domain/library_organization_history.dart";

enum _OrganizationMenuAction { undo, redo, ignore }

class LibraryOrganizationMenu extends ConsumerStatefulWidget {
  const LibraryOrganizationMenu({
    required this.enabled,
    this.onIgnore,
    this.ignoreLabel = "忽略此图",
    super.key,
  });

  final bool enabled;
  final VoidCallback? onIgnore;
  final String ignoreLabel;

  @override
  ConsumerState<LibraryOrganizationMenu> createState() =>
      _LibraryOrganizationMenuState();
}

class _LibraryOrganizationMenuState
    extends ConsumerState<LibraryOrganizationMenu> {
  LibraryOrganizationHistory? _openedHistory;

  @override
  Widget build(BuildContext context) {
    final result = ref.watch(libraryOrganizationHistoryProvider);
    final saving = ref.watch(libraryOrganizationActionsProvider).saving;
    final history = result.isLoading || result.hasError ? null : result.value;
    final canAct = widget.enabled && !saving;
    void step(LibraryOrganizationHistoryDirection direction) {
      final observed = _openedHistory;
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
      onOpened: () => _openedHistory = history,
      onOpenChanged: (open) {
        if (!open) _openedHistory = null;
      },
      labels: ["撤销操作", "重做操作", widget.ignoreLabel],
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
        if (widget.onIgnore != null) ...[
          const PopupMenuDivider(),
          PopupMenuItem(
            value: _OrganizationMenuAction.ignore,
            enabled: canAct,
            child: AmeMenuItemContent(
              label: widget.ignoreLabel,
              icon: Symbols.hide_image_rounded,
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
          case _OrganizationMenuAction.ignore:
            if (canAct) widget.onIgnore?.call();
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
