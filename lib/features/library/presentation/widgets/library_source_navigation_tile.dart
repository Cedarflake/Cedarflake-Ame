import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";

import "../../../../app/ame_menu.dart";
import "../../../../app/ame_popup_menu_position.dart";
import "../../domain/library_models.dart";
import "../library_strings.dart";
import "library_path_text.dart";

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
    final tile = widget.isCompact
        ? IconButton(
            focusNode: _focusNode,
            isSelected: widget.isSelected,
            tooltip: widget.root.displayPath,
            onPressed: widget.isBusy ? null : widget.onSelect,
            icon: Icon(icon),
          )
        : _ExpandedSourceTile(
            focusNode: _focusNode,
            icon: icon,
            iconKey: ValueKey("source-icon-${widget.root.id}"),
            title: librarySourceName(widget.root.displayPath),
            path: widget.root.displayPath,
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
                  Builder(
                    builder: (buttonContext) => IconButton(
                      key: ValueKey("source-more-${widget.root.id}"),
                      tooltip: LibraryStrings.more,
                      onPressed: () {
                        unawaited(_showMenuBelow(buttonContext));
                      },
                      icon: const Icon(Icons.more_vert),
                    ),
                  ),
                ],
              ),
            ),
            onTap: widget.isBusy ? null : widget.onSelect,
          );
    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.contextMenu): () {
          final anchorContext = _focusNode.context;
          if (anchorContext != null) {
            unawaited(_showMenuBelow(anchorContext));
          }
        },
        const SingleActivator(LogicalKeyboardKey.f10, shift: true): () {
          final anchorContext = _focusNode.context;
          if (anchorContext != null) {
            unawaited(_showMenuBelow(anchorContext));
          }
        },
      },
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onSecondaryTapDown: (details) {
          unawaited(_showMenuAtGlobalPosition(details.globalPosition));
        },
        child: tile,
      ),
    );
  }

  Future<void> _showMenuBelow(BuildContext anchorContext) async {
    final position = amePopupMenuBelowAnchor(
      context: context,
      anchorContext: anchorContext,
    );
    if (position != null) {
      await _showMenu(position);
    }
  }

  Future<void> _showMenuAtGlobalPosition(Offset globalPosition) async {
    _focusNode.requestFocus();
    final position = amePopupMenuAtGlobalPosition(
      context: context,
      globalPosition: globalPosition,
    );
    if (position != null) {
      await _showMenu(position);
    }
  }

  Future<void> _showMenu(RelativeRect position) async {
    const labels = [
      LibraryStrings.updateLibrary,
      LibraryStrings.openInExplorer,
      LibraryStrings.removeFromAme,
    ];
    final action = await showAmePopupMenu<_LibrarySourceMenuAction>(
      context: context,
      position: position,
      labels: labels,
      items: [
        PopupMenuItem(
          value: _LibrarySourceMenuAction.update,
          enabled: !widget.isBusy,
          child: const AmeMenuItemContent(
            icon: Icons.refresh,
            label: LibraryStrings.updateLibrary,
          ),
        ),
        const PopupMenuItem(
          value: _LibrarySourceMenuAction.open,
          child: AmeMenuItemContent(
            icon: Icons.folder_open_outlined,
            label: LibraryStrings.openInExplorer,
          ),
        ),
        const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
        PopupMenuItem(
          value: _LibrarySourceMenuAction.remove,
          enabled: !widget.isBusy,
          child: const AmeMenuItemContent(
            icon: Icons.remove_circle_outline,
            label: LibraryStrings.removeFromAme,
          ),
        ),
      ],
    );
    if (!mounted || action == null) {
      return;
    }
    switch (action) {
      case _LibrarySourceMenuAction.update:
        widget.onUpdate();
      case _LibrarySourceMenuAction.open:
        widget.onOpen();
      case _LibrarySourceMenuAction.remove:
        widget.onRemove();
    }
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

enum _LibrarySourceMenuAction { update, open, remove }

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
      path: path,
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
      .split("\\")
      .where((segment) => segment.isNotEmpty)
      .toList(growable: false);
  return segments.isEmpty ? path : segments.last;
}

class _ExpandedSourceTile extends StatelessWidget {
  const _ExpandedSourceTile({
    required this.icon,
    required this.title,
    required this.path,
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
  final String path;
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
      title: LibraryPathText(text: title, path: path, textKey: titleKey),
      subtitle: subtitle == null
          ? null
          : Text(subtitle!, maxLines: 1, overflow: TextOverflow.ellipsis),
      trailing: trailing,
      onTap: onTap,
    );
  }
}
