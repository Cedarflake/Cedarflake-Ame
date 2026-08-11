import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_menu.dart";
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
  final MenuController _menuController = MenuController();
  final FocusNode _focusNode = FocusNode(debugLabel: "Library folder");

  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final keySuffix = "${widget.root.id}-${widget.folder.relativePath}";
    return AmeMenuAnchor(
      controller: _menuController,
      childFocusNode: _focusNode,
      menuChildren: [
        MenuItemButton(
          onPressed: widget.onOpen,
          child: const AmeMenuItemContent(
            icon: Symbols.folder_open_rounded,
            label: LibraryStrings.openInExplorer,
          ),
        ),
      ],
      child: CallbackShortcuts(
        bindings: {
          const SingleActivator(LogicalKeyboardKey.contextMenu):
              _menuController.open,
          const SingleActivator(LogicalKeyboardKey.f10, shift: true):
              _menuController.open,
        },
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onSecondaryTapDown: (details) {
            _focusNode.requestFocus();
            _menuController.open(position: details.localPosition);
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
            title: LibraryPathText(
              text: widget.folder.name,
              path: displayLibraryFolderPath(
                widget.root.displayPath,
                widget.folder.relativePath,
              ),
              textKey: ValueKey("folder-title-$keySuffix"),
            ),
            trailing: SizedBox(
              width: 48,
              child: widget.onToggleExpansion == null
                  ? null
                  : IconButton(
                      key: ValueKey("folder-expand-$keySuffix"),
                      tooltip: widget.isExpanded
                          ? LibraryStrings.collapseFolder
                          : LibraryStrings.expandFolder,
                      onPressed: widget.onToggleExpansion,
                      icon: Icon(
                        widget.isExpanded
                            ? Symbols.keyboard_arrow_up_rounded
                            : Symbols.keyboard_arrow_down_rounded,
                      ),
                    ),
            ),
            onTap: widget.isBusy ? null : widget.onSelect,
          ),
        ),
      ),
    );
  }
}

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
