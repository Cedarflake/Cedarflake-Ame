import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_popup_menu_position.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../domain/library_folder_models.dart";
import "../../domain/library_models.dart";
import "../library_strings.dart";
import "library_path_text.dart";

class LibraryFolderNavigationTile extends StatefulWidget {
  const LibraryFolderNavigationTile({
    required this.root,
    required this.folder,
    required this.depth,
    required this.isSelected,
    required this.isExpanded,
    required this.isBusy,
    required this.onSelect,
    required this.onToggleExpansion,
    required this.onOpen,
    super.key,
  });

  final LibraryRoot root;
  final LibraryFolder folder;
  final int depth;
  final bool isSelected;
  final bool isExpanded;
  final bool isBusy;
  final VoidCallback onSelect;
  final VoidCallback? onToggleExpansion;
  final VoidCallback onOpen;

  @override
  State<LibraryFolderNavigationTile> createState() =>
      _LibraryFolderNavigationTileState();
}

class _LibraryFolderNavigationTileState
    extends State<LibraryFolderNavigationTile> {
  final FocusNode _focusNode = FocusNode(debugLabel: "Library folder");

  String get _keySuffix => "${widget.root.id}-${widget.folder.relativePath}";

  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final keySuffix = _keySuffix;
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.contextMenu):
            _openKeyboardMenu,
        const SingleActivator(LogicalKeyboardKey.f10, shift: true):
            _openKeyboardMenu,
      },
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onSecondaryTapDown: (details) {
          _focusNode.requestFocus();
          final position = amePopupMenuAtGlobalPosition(
            context: context,
            globalPosition: details.globalPosition,
          );
          if (position != null) {
            unawaited(_showContextMenu(position));
          }
        },
        child: ListTile(
          key: ValueKey("folder-tile-$keySuffix"),
          focusNode: _focusNode,
          selected: widget.isSelected,
          selectedTileColor: Theme.of(context).colorScheme.secondaryContainer,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
          ),
          dense: true,
          contentPadding: EdgeInsets.only(
            left: 16 + (widget.depth * 28),
            right: 4,
          ),
          minLeadingWidth: 24,
          horizontalTitleGap: 12,
          leading: const SizedBox(
            width: 24,
            child: Icon(Symbols.folder_rounded),
          ),
          title: DefaultTextStyle.merge(
            style: TextStyle(
              fontWeight: widget.isSelected
                  ? ameFontWeightSemibold
                  : ameFontWeightMedium,
            ),
            child: LibraryPathText(
              text: widget.folder.name,
              path: displayLibraryFolderPath(
                widget.root.displayPath,
                widget.folder.relativePath,
              ),
              textKey: ValueKey("folder-title-$keySuffix"),
            ),
          ),
          trailing: SizedBox(
            width: 48,
            child: widget.onToggleExpansion == null
                ? null
                : AmeTooltip(
                    message: widget.isExpanded
                        ? LibraryStrings.collapseFolder
                        : LibraryStrings.expandFolder,
                    child: IconButton(
                      key: ValueKey("folder-expand-$keySuffix"),
                      onPressed: widget.onToggleExpansion,
                      icon: Icon(
                        widget.isExpanded
                            ? Symbols.keyboard_arrow_up_rounded
                            : Symbols.keyboard_arrow_down_rounded,
                      ),
                    ),
                  ),
          ),
          onTap: widget.isBusy ? null : widget.onSelect,
        ),
      ),
    );
  }

  void _openKeyboardMenu() {
    final anchorContext = _focusNode.context;
    if (anchorContext == null) {
      return;
    }
    final position = amePopupMenuBelowAnchor(
      context: context,
      anchorContext: anchorContext,
    );
    if (position != null) {
      unawaited(_showContextMenu(position));
    }
  }

  Future<void> _showContextMenu(RelativeRect position) async {
    final targetKey = _keySuffix;
    final onOpen = widget.onOpen;
    final action = await showAmePopupMenu<_LibraryFolderMenuAction>(
      context: context,
      position: position,
      labels: const [LibraryStrings.openInExplorer],
      items: const [
        PopupMenuItem(
          value: _LibraryFolderMenuAction.open,
          child: AmeMenuItemContent(
            icon: Symbols.folder_open_rounded,
            label: LibraryStrings.openInExplorer,
          ),
        ),
      ],
    );
    if (mounted &&
        _keySuffix == targetKey &&
        action == _LibraryFolderMenuAction.open) {
      onOpen();
    }
  }
}

enum _LibraryFolderMenuAction { open }

class LibraryFolderLoadingTile extends StatelessWidget {
  const LibraryFolderLoadingTile({required this.depth, super.key});

  final int depth;

  @override
  Widget build(BuildContext context) {
    return Padding(
      key: const Key("folder-tree-loading"),
      padding: EdgeInsets.only(left: 28 + (depth * 28), right: 16),
      child: const SizedBox(
        height: 40,
        child: Align(
          alignment: Alignment.centerLeft,
          child: SizedBox.square(
            dimension: 18,
            child: CircularProgressIndicator(strokeWidth: 2.5),
          ),
        ),
      ),
    );
  }
}

class LibraryFolderErrorTile extends StatelessWidget {
  const LibraryFolderErrorTile({
    required this.depth,
    required this.onRetry,
    super.key,
  });

  final int depth;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.only(left: 16 + (depth * 28), right: 4),
      child: TextButton.icon(
        key: const Key("folder-tree-retry"),
        onPressed: onRetry,
        icon: const Icon(Symbols.refresh_rounded),
        label: const Text(LibraryStrings.retryFolders),
      ),
    );
  }
}

class LoadMoreLibraryFoldersTile extends StatelessWidget {
  const LoadMoreLibraryFoldersTile({
    required this.depth,
    required this.isLoading,
    required this.onPressed,
    super.key,
  });

  final int depth;
  final bool isLoading;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.only(left: 16 + (depth * 28), right: 4),
      child: TextButton(
        key: const Key("folder-tree-load-more"),
        onPressed: isLoading ? null : onPressed,
        child: Text(
          isLoading
              ? LibraryStrings.loadingFolders
              : LibraryStrings.showMoreFolders,
        ),
      ),
    );
  }
}
