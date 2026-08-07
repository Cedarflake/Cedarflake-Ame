import "package:flutter/material.dart";
import "package:flutter/services.dart";

import "../../domain/library_models.dart";
import "../library_strings.dart";

class LibrarySourceNavigationTile extends StatefulWidget {
  const LibrarySourceNavigationTile({
    required this.root,
    required this.isCompact,
    required this.isSelected,
    required this.isExpanded,
    required this.isBusy,
    required this.onSelect,
    required this.onToggleExpansion,
    required this.onUpdate,
    required this.onOpen,
    required this.onRemove,
    super.key,
  });

  final LibraryRoot root;
  final bool isCompact;
  final bool isSelected;
  final bool isExpanded;
  final bool isBusy;
  final VoidCallback onSelect;
  final VoidCallback onToggleExpansion;
  final VoidCallback onUpdate;
  final VoidCallback onOpen;
  final VoidCallback onRemove;

  @override
  State<LibrarySourceNavigationTile> createState() =>
      _LibrarySourceNavigationTileState();
}

class _LibrarySourceNavigationTileState
    extends State<LibrarySourceNavigationTile> {
  final MenuController _menuController = MenuController();
  final FocusNode _focusNode = FocusNode(debugLabel: "Library source");

  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final icon = switch (widget.root.availability) {
      LibraryRootAvailability.available ||
      LibraryRootAvailability.unknown => Icons.folder_outlined,
      LibraryRootAvailability.missing => Icons.folder_off_outlined,
      LibraryRootAvailability.inaccessible => Icons.lock_outline,
      LibraryRootAvailability.offline => Icons.cloud_off_outlined,
    };
    final menuChildren = [
      MenuItemButton(
        leadingIcon: const Icon(Icons.refresh),
        onPressed: widget.isBusy ? null : widget.onUpdate,
        child: const Text(LibraryStrings.updateLibrary),
      ),
      MenuItemButton(
        leadingIcon: const Icon(Icons.folder_open_outlined),
        onPressed: widget.onOpen,
        child: const Text(LibraryStrings.openInExplorer),
      ),
      const Divider(height: 1),
      MenuItemButton(
        leadingIcon: const Icon(Icons.remove_circle_outline),
        onPressed: widget.isBusy ? null : widget.onRemove,
        child: const Text(LibraryStrings.removeFromAme),
      ),
    ];
    final tile = widget.isCompact
        ? IconButton(
            focusNode: _focusNode,
            isSelected: widget.isSelected,
            tooltip: widget.root.path,
            onPressed: widget.isBusy ? null : widget.onSelect,
            icon: Icon(icon),
          )
        : _ExpandedSourceTile(
            focusNode: _focusNode,
            icon: icon,
            iconKey: ValueKey("source-icon-${widget.root.id}"),
            title: librarySourceName(widget.root.path),
            titleKey: ValueKey("source-title-${widget.root.id}"),
            subtitle:
                widget.root.availability == LibraryRootAvailability.available
                ? null
                : _availabilityLabel(widget.root.availability),
            isSelected: widget.isSelected,
            trailing: SizedBox(
              width: 96,
              child: Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: [
                  IconButton(
                    key: ValueKey("source-expand-${widget.root.id}"),
                    tooltip: widget.isExpanded
                        ? LibraryStrings.collapseFolder
                        : LibraryStrings.expandFolder,
                    onPressed: widget.onToggleExpansion,
                    icon: Icon(
                      widget.isExpanded
                          ? Icons.keyboard_arrow_up
                          : Icons.keyboard_arrow_down,
                    ),
                  ),
                  IconButton(
                    key: ValueKey("source-more-${widget.root.id}"),
                    tooltip: LibraryStrings.more,
                    onPressed: _menuController.open,
                    icon: const Icon(Icons.more_vert),
                  ),
                ],
              ),
            ),
            onTap: widget.isBusy ? null : widget.onSelect,
          );
    return MenuAnchor(
      controller: _menuController,
      childFocusNode: _focusNode,
      menuChildren: menuChildren,
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
          child: tile,
        ),
      ),
    );
  }

  static String _availabilityLabel(LibraryRootAvailability availability) {
    return switch (availability) {
      LibraryRootAvailability.available => "可用",
      LibraryRootAvailability.missing => "文件夹不存在",
      LibraryRootAvailability.inaccessible => "无法访问",
      LibraryRootAvailability.offline => "当前离线",
      LibraryRootAvailability.unknown => "状态未知",
    };
  }
}

class PendingLibrarySourceTile extends StatelessWidget {
  const PendingLibrarySourceTile({
    required this.path,
    required this.isCompact,
    super.key,
  });

  final String path;
  final bool isCompact;

  @override
  Widget build(BuildContext context) {
    if (isCompact) {
      return Tooltip(
        message: path,
        child: const IconButton(
          onPressed: null,
          icon: Stack(
            clipBehavior: Clip.none,
            children: [
              Icon(Icons.folder_outlined),
              Positioned(
                right: -5,
                bottom: -5,
                child: SizedBox.square(
                  dimension: 12,
                  child: CircularProgressIndicator(strokeWidth: 2),
                ),
              ),
            ],
          ),
        ),
      );
    }
    return _ExpandedSourceTile(
      key: const Key("pending-source-tile"),
      icon: Icons.folder_outlined,
      iconKey: const Key("pending-source-icon"),
      title: librarySourceName(path),
      titleKey: const Key("pending-source-title"),
      subtitle: "正在添加",
      trailing: const SizedBox(
        width: 96,
        child: Align(
          alignment: Alignment.centerRight,
          child: SizedBox(
            width: 48,
            child: Center(
              child: SizedBox.square(
                key: Key("pending-source-progress"),
                dimension: 18,
                child: CircularProgressIndicator(strokeWidth: 2.5),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

String librarySourceName(String path) {
  final segments = path
      .replaceAll("/", "\\")
      .split("\\")
      .where((segment) => segment.isNotEmpty)
      .toList(growable: false);
  return segments.isEmpty ? path : segments.last;
}

class _ExpandedSourceTile extends StatelessWidget {
  const _ExpandedSourceTile({
    required this.icon,
    required this.title,
    required this.trailing,
    this.iconKey,
    this.titleKey,
    this.subtitle,
    this.isSelected = false,
    this.focusNode,
    this.onTap,
    super.key,
  });

  final IconData icon;
  final Key? iconKey;
  final String title;
  final Key? titleKey;
  final String? subtitle;
  final bool isSelected;
  final Widget trailing;
  final FocusNode? focusNode;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      focusNode: focusNode,
      selected: isSelected,
      selectedTileColor: Theme.of(context).colorScheme.secondaryContainer,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      dense: true,
      contentPadding: const EdgeInsets.only(left: 16, right: 4),
      minLeadingWidth: 24,
      horizontalTitleGap: 12,
      leading: SizedBox(key: iconKey, width: 24, child: Icon(icon)),
      title: Text(
        title,
        key: titleKey,
        maxLines: 1,
        overflow: TextOverflow.ellipsis,
      ),
      subtitle: subtitle == null
          ? null
          : Text(subtitle!, maxLines: 1, overflow: TextOverflow.ellipsis),
      trailing: trailing,
      onTap: onTap,
    );
  }
}
