import "dart:async";

import "package:flutter/material.dart";

import "../../application/library_folder_controller.dart";
import "../../domain/library_folder_models.dart";
import "../../domain/library_models.dart";
import "../library_strings.dart";
import "library_folder_navigation_tile.dart";
import "library_source_navigation_tile.dart";

export "library_source_navigation_tile.dart" show librarySourceName;

class LibraryNavigation extends StatefulWidget {
  const LibraryNavigation({
    required this.isCompact,
    required this.width,
    required this.isSettingsSelected,
    required this.roots,
    required this.selectedRootId,
    required this.selectedFolderRelativePath,
    required this.transientRootPath,
    required this.folderTree,
    required this.isBusy,
    required this.onSelectLibrary,
    required this.onSelectRoot,
    required this.onSelectFolder,
    required this.onExpandFolder,
    required this.onLoadMoreFolders,
    required this.onAddSource,
    required this.onOpenSettings,
    required this.onUpdateRoot,
    required this.onOpenRoot,
    required this.onOpenFolder,
    required this.onRemoveRoot,
    super.key,
  });

  final bool isCompact;
  final double width;
  final bool isSettingsSelected;
  final List<LibraryRoot> roots;
  final String? selectedRootId;
  final String? selectedFolderRelativePath;
  final String? transientRootPath;
  final LibraryFolderTreeState folderTree;
  final bool isBusy;
  final VoidCallback onSelectLibrary;
  final ValueChanged<LibraryRoot> onSelectRoot;
  final void Function(LibraryRoot root, LibraryFolder folder) onSelectFolder;
  final Future<void> Function(String rootId, String parentRelativePath)
  onExpandFolder;
  final Future<void> Function(String rootId, String parentRelativePath)
  onLoadMoreFolders;
  final VoidCallback onAddSource;
  final VoidCallback onOpenSettings;
  final ValueChanged<String> onUpdateRoot;
  final ValueChanged<LibraryRoot> onOpenRoot;
  final void Function(LibraryRoot root, LibraryFolder folder) onOpenFolder;
  final ValueChanged<LibraryRoot> onRemoveRoot;

  @override
  State<LibraryNavigation> createState() => _LibraryNavigationState();
}

class _LibraryNavigationState extends State<LibraryNavigation> {
  final Set<LibraryFolderBranchKey> _expandedBranches = {};

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      key: const Key("library-navigation"),
      width: widget.width,
      child: Material(
        key: const Key("library-navigation-surface"),
        color: Theme.of(context).colorScheme.surfaceContainerLow,
        child: Column(
          children: [
            Expanded(
              child: ListView(
                padding: const EdgeInsets.fromLTRB(8, 12, 8, 12),
                children: [
                  if (widget.isCompact) ...[
                    IconButton(
                      key: const Key("library-sidebar-library"),
                      isSelected:
                          !widget.isSettingsSelected &&
                          widget.selectedRootId == null,
                      tooltip: LibraryStrings.library,
                      onPressed: widget.isBusy ? null : widget.onSelectLibrary,
                      icon: const Icon(Icons.photo_library_outlined),
                    ),
                    IconButton(
                      key: const Key("library-sidebar-import"),
                      tooltip: LibraryStrings.addFolder,
                      onPressed: widget.isBusy ? null : widget.onAddSource,
                      icon: const Icon(Icons.create_new_folder_outlined),
                    ),
                  ] else
                    ListTile(
                      key: const Key("library-sidebar-library"),
                      selected:
                          !widget.isSettingsSelected &&
                          widget.selectedRootId == null,
                      selectedTileColor: Theme.of(
                        context,
                      ).colorScheme.secondaryContainer,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(12),
                      ),
                      contentPadding: const EdgeInsets.only(left: 16, right: 4),
                      minLeadingWidth: 24,
                      horizontalTitleGap: 12,
                      leading: const SizedBox(
                        width: 24,
                        child: Icon(Icons.photo_library_outlined),
                      ),
                      title: const Text(LibraryStrings.library),
                      trailing: SizedBox(
                        width: 48,
                        child: IconButton(
                          key: const Key("library-sidebar-import"),
                          tooltip: LibraryStrings.addFolder,
                          onPressed: widget.isBusy ? null : widget.onAddSource,
                          icon: const Icon(Icons.create_new_folder_outlined),
                        ),
                      ),
                      onTap: widget.isBusy ? null : widget.onSelectLibrary,
                    ),
                  const SizedBox(height: 12),
                  if (widget.roots.isEmpty && widget.transientRootPath == null)
                    if (widget.isCompact)
                      const IconButton(
                        tooltip: LibraryStrings.noFolder,
                        onPressed: null,
                        icon: Icon(Icons.folder_off_outlined),
                      )
                    else
                      const ListTile(
                        leading: Icon(Icons.folder_off_outlined),
                        title: Text(LibraryStrings.noFolder),
                      ),
                  for (final root in widget.roots) ...[
                    LibrarySourceNavigationTile(
                      root: root,
                      isCompact: widget.isCompact,
                      isSelected:
                          !widget.isSettingsSelected &&
                          widget.selectedRootId == root.id &&
                          widget.selectedFolderRelativePath == null,
                      isExpanded: _isExpanded(root.id, ""),
                      isBusy: widget.isBusy,
                      onSelect: () => widget.onSelectRoot(root),
                      onToggleExpansion: () => _toggleBranch(root.id, ""),
                      onUpdate: () => widget.onUpdateRoot(root.path),
                      onOpen: () => widget.onOpenRoot(root),
                      onRemove: () => widget.onRemoveRoot(root),
                    ),
                    if (!widget.isCompact && _isExpanded(root.id, ""))
                      ..._buildFolderBranch(root, "", 1),
                  ],
                  if (widget.transientRootPath case final path?)
                    PendingLibrarySourceTile(
                      path: path,
                      isCompact: widget.isCompact,
                    ),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.fromLTRB(8, 12, 8, 12),
              child: widget.isCompact
                  ? IconButton(
                      key: const Key("library-sidebar-settings"),
                      isSelected: widget.isSettingsSelected,
                      tooltip: LibraryStrings.settings,
                      onPressed: widget.onOpenSettings,
                      icon: const Icon(Icons.settings_outlined),
                    )
                  : ListTile(
                      key: const Key("library-sidebar-settings"),
                      selected: widget.isSettingsSelected,
                      selectedTileColor: Theme.of(
                        context,
                      ).colorScheme.secondaryContainer,
                      shape: RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(12),
                      ),
                      contentPadding: const EdgeInsets.symmetric(
                        horizontal: 16,
                      ),
                      minLeadingWidth: 24,
                      horizontalTitleGap: 12,
                      leading: const SizedBox(
                        width: 24,
                        child: Icon(Icons.settings_outlined),
                      ),
                      title: const Text(LibraryStrings.settings),
                      onTap: widget.onOpenSettings,
                    ),
            ),
          ],
        ),
      ),
    );
  }

  List<Widget> _buildFolderBranch(
    LibraryRoot root,
    String parentRelativePath,
    int depth,
  ) {
    final branch = widget.folderTree.branch(root.id, parentRelativePath);
    if (branch.isLoading && !branch.hasLoaded) {
      return [LibraryFolderLoadingTile(depth: depth)];
    }
    if (branch.errorMessage != null && !branch.hasLoaded) {
      return [
        LibraryFolderErrorTile(
          depth: depth,
          onRetry: () =>
              unawaited(widget.onExpandFolder(root.id, parentRelativePath)),
        ),
      ];
    }

    final children = <Widget>[];
    for (final folder in branch.folders) {
      final isExpanded = _isExpanded(root.id, folder.relativePath);
      children.add(
        LibraryFolderNavigationTile(
          root: root,
          folder: folder,
          depth: depth,
          isSelected:
              !widget.isSettingsSelected &&
              widget.selectedRootId == root.id &&
              widget.selectedFolderRelativePath == folder.relativePath,
          isExpanded: isExpanded,
          isBusy: widget.isBusy,
          onSelect: () => widget.onSelectFolder(root, folder),
          onToggleExpansion: folder.hasChildFolders
              ? () => _toggleBranch(root.id, folder.relativePath)
              : null,
          onOpen: () => widget.onOpenFolder(root, folder),
        ),
      );
      if (isExpanded) {
        children.addAll(
          _buildFolderBranch(root, folder.relativePath, depth + 1),
        );
      }
    }
    if (branch.hasMore) {
      children.add(
        LoadMoreLibraryFoldersTile(
          depth: depth,
          isLoading: branch.isLoading,
          onPressed: () =>
              unawaited(widget.onLoadMoreFolders(root.id, parentRelativePath)),
        ),
      );
    }
    return children;
  }

  bool _isExpanded(String rootId, String parentRelativePath) {
    return _expandedBranches.contains(
      LibraryFolderBranchKey(
        rootId: rootId,
        parentRelativePath: parentRelativePath,
      ),
    );
  }

  void _toggleBranch(String rootId, String parentRelativePath) {
    final key = LibraryFolderBranchKey(
      rootId: rootId,
      parentRelativePath: parentRelativePath,
    );
    if (_expandedBranches.contains(key)) {
      setState(() => _expandedBranches.remove(key));
      return;
    }
    setState(() => _expandedBranches.add(key));
    unawaited(widget.onExpandFolder(rootId, parentRelativePath));
  }
}
