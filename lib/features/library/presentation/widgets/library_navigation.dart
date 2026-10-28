import "dart:async";

import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../application/library_folder_controller.dart";
import "../../application/library_sidebar_order.dart";
import "../../domain/library_folder_models.dart";
import "../../domain/library_models.dart";
import "../../domain/library_synchronization_models.dart";
import "../library_pending_source.dart";
import "../library_strings.dart";
import "../library_sidebar_selection.dart";
import "library_folder_navigation_tile.dart";
import "library_source_navigation_tile.dart";
import "library_sidebar_section_header.dart";
import "library_sidebar_reorder_list.dart";
import "library_sidebar_selection_region.dart";

export "library_source_navigation_tile.dart" show librarySourceName;

class LibraryNavigation extends StatefulWidget {
  const LibraryNavigation({
    required this.isCompact,
    required this.width,
    required this.roots,
    this.rootSynchronizationStatuses = const {},
    this.hasSynchronizationFailure = false,
    required this.selectedRootId,
    required this.selectedFolderRelativePath,
    required this.transientSource,
    required this.folderTree,
    required this.isBusy,
    required this.isBrowseDisabled,
    this.updatingRootIds = const {},
    this.isAddingSourceDisabled = false,
    this.addingSourceDisabledReason,
    this.retainedScanRootId,
    required this.onSelectLibrary,
    required this.onSelectRoot,
    required this.onSelectFolder,
    required this.onExpandFolder,
    required this.onLoadMoreFolders,
    required this.onAddSource,
    required this.onUpdateRoot,
    this.onRelocateRoot,
    required this.onOpenRoot,
    required this.onOpenFolder,
    required this.onRemoveRoot,
    this.onRemoveRoots,
    this.isActive = true,
    this.onReorder,
    this.collection = const LibraryGalleryCollection.library(),
    super.key,
  });

  final bool isCompact;
  final double width;
  final List<LibraryRoot> roots;
  final Map<String, LibraryRootSynchronizationStatus>
  rootSynchronizationStatuses;
  final bool hasSynchronizationFailure;
  final String? selectedRootId;
  final String? selectedFolderRelativePath;
  final LibraryPendingSource? transientSource;
  final LibraryFolderTreeState folderTree;
  final bool isBusy;
  final bool isBrowseDisabled;
  final Set<String> updatingRootIds;
  final bool isAddingSourceDisabled;
  final String? addingSourceDisabledReason;
  final String? retainedScanRootId;
  final VoidCallback onSelectLibrary;
  final ValueChanged<LibraryRoot> onSelectRoot;
  final void Function(LibraryRoot root, LibraryFolder folder) onSelectFolder;
  final Future<void> Function(String rootId, String parentRelativePath)
  onExpandFolder;
  final Future<void> Function(String rootId, String parentRelativePath)
  onLoadMoreFolders;
  final VoidCallback onAddSource;
  final ValueChanged<LibraryRoot> onUpdateRoot;
  final ValueChanged<LibraryRoot>? onRelocateRoot;
  final ValueChanged<LibraryRoot> onOpenRoot;
  final void Function(LibraryRoot root, LibraryFolder folder) onOpenFolder;
  final ValueChanged<LibraryRoot> onRemoveRoot;
  final Future<Set<String>> Function(List<LibraryRoot> roots)? onRemoveRoots;
  final bool isActive;
  final Future<void> Function(LibrarySidebarMove move)? onReorder;
  final LibraryGalleryCollection collection;

  @override
  State<LibraryNavigation> createState() => _LibraryNavigationState();
}

class _LibraryNavigationState extends State<LibraryNavigation> {
  final Set<LibraryFolderBranchKey> _expandedBranches = {};
  final FocusNode _libraryFocusNode = FocusNode(
    debugLabel: "Library navigation entry",
  );
  final Map<String, FocusNode> _rootFocusNodes = {};
  String? _pendingRemovalFocusRootId;
  final _selection = LibrarySidebarSelection();
  bool _removing = false;

  Future<void> _removeSelected() async {
    final remove = widget.onRemoveRoots;
    if (remove == null || _removing) return;
    final roots = widget.roots
        .where((root) => _selection.contains(root.id))
        .toList();
    if (roots.isEmpty) return;
    setState(() => _removing = true);
    final completed = await remove(roots);
    if (!mounted) return;
    setState(() {
      _removing = false;
      _selection.removeAll(completed);
      _selection.retain(widget.roots.map((root) => root.id));
    });
  }

  @override
  void didUpdateWidget(covariant LibraryNavigation oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!widget.isActive) _selection.clear();
    final currentRootIds = widget.roots.map((root) => root.id).toSet();
    if (!_removing) _selection.retain(currentRootIds);
    if (widget.isCompact) _selection.clear();
    _expandedBranches.removeWhere(
      (key) =>
          !currentRootIds.contains(key.rootId) ||
          (oldWidget.folderTree.branches.containsKey(key) &&
              !widget.folderTree.branches.containsKey(key)),
    );
    final removedRootIds = oldWidget.roots
        .map((root) => root.id)
        .where((rootId) => !currentRootIds.contains(rootId))
        .toList(growable: false);
    if (removedRootIds.isEmpty) {
      return;
    }
    String? focusedRemovedRootId;
    final pendingRootId = _pendingRemovalFocusRootId;
    if (pendingRootId != null && removedRootIds.contains(pendingRootId)) {
      focusedRemovedRootId = pendingRootId;
    } else {
      for (final rootId in removedRootIds) {
        if (_rootFocusNodes[rootId]?.hasFocus ?? false) {
          focusedRemovedRootId = rootId;
          break;
        }
      }
    }
    final nodesToDispose = [
      for (final rootId in removedRootIds) ?_rootFocusNodes.remove(rootId),
    ];
    _pendingRemovalFocusRootId = null;
    if (focusedRemovedRootId == null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        for (final node in nodesToDispose) {
          node.dispose();
        }
      });
      return;
    }
    final removedIndex = oldWidget.roots.indexWhere(
      (root) => root.id == focusedRemovedRootId,
    );
    final targetRoot = widget.roots.isEmpty
        ? null
        : widget.roots[removedIndex.clamp(0, widget.roots.length - 1).toInt()];
    final targetNode = targetRoot == null
        ? _libraryFocusNode
        : _focusNodeForRoot(targetRoot.id);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        targetNode.requestFocus();
      }
      for (final node in nodesToDispose) {
        node.dispose();
      }
    });
  }

  @override
  void dispose() {
    _libraryFocusNode.dispose();
    for (final node in _rootFocusNodes.values) {
      node.dispose();
    }
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return LibrarySidebarSelectionRegion(
      onClear: _selection.isActive && !_removing
          ? () => setState(_selection.clear)
          : null,
      child: SizedBox(
        key: const Key("library-navigation"),
        width: widget.width,
        child: Material(
          key: const Key("library-navigation-surface"),
          color: Theme.of(context).colorScheme.surfaceContainerLow,
          child: Column(
            children: [
              Expanded(
                child: LibrarySidebarReorderList<LibraryRoot>(
                  items: widget.roots,
                  itemId: (root) => root.id,
                  onReorder: widget.onReorder,
                  enabled:
                      !widget.isBusy &&
                      !widget.isBrowseDisabled &&
                      !_selection.isActive &&
                      !_removing,
                  header: (saving) => Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      if (widget.isCompact) ...[
                        AmeTooltip(
                          message: LibraryStrings.library,
                          child: IconButton(
                            key: const Key("library-sidebar-library"),
                            focusNode: _libraryFocusNode,
                            isSelected:
                                widget.selectedRootId == null &&
                                widget.collection is LibraryAllCollection,
                            onPressed: widget.isBrowseDisabled
                                ? null
                                : widget.onSelectLibrary,
                            icon: const Icon(Symbols.photo_library_rounded),
                          ),
                        ),
                        AmeTooltip(
                          message:
                              widget.addingSourceDisabledReason ??
                              LibraryStrings.addFolder,
                          child: IconButton(
                            key: const Key("library-sidebar-import"),
                            onPressed:
                                widget.isBusy || widget.isAddingSourceDisabled
                                ? null
                                : widget.onAddSource,
                            icon: const Icon(Symbols.create_new_folder_rounded),
                          ),
                        ),
                      ] else
                        ListTile(
                          key: const Key("library-sidebar-library"),
                          focusNode: _libraryFocusNode,
                          selected:
                              widget.selectedRootId == null &&
                              widget.collection is LibraryAllCollection,
                          selectedTileColor: Theme.of(
                            context,
                          ).colorScheme.secondaryContainer,
                          shape: RoundedRectangleBorder(
                            borderRadius: BorderRadius.circular(12),
                          ),
                          contentPadding: const EdgeInsets.only(
                            left: 16,
                            right: 4,
                          ),
                          minLeadingWidth: 24,
                          horizontalTitleGap: 12,
                          leading: const SizedBox(
                            width: 24,
                            child: Icon(Symbols.photo_library_rounded),
                          ),
                          title: Text(
                            LibraryStrings.library,
                            style: _navigationTitleStyle(
                              context,
                              widget.selectedRootId == null &&
                                  widget.collection is LibraryAllCollection,
                            ),
                          ),
                          onTap: widget.isBrowseDisabled
                              ? null
                              : widget.onSelectLibrary,
                        ),
                      if (!widget.isCompact)
                        LibrarySidebarSectionHeader(
                          title: "文件夹",
                          selectedCount: _selection.count,
                          allSelected: _selection.allSelected(
                            widget.roots.map((root) => root.id),
                          ),
                          onSelectAll: _removing
                              ? null
                              : () => setState(
                                  () => _selection.toggleAll(
                                    widget.roots.map((root) => root.id),
                                  ),
                                ),
                          removeLabel: "从 Ame 中移除所选文件夹",
                          onRemove: widget.isBusy || _removing
                              ? null
                              : _removeSelected,
                          createLabel:
                              widget.addingSourceDisabledReason ??
                              LibraryStrings.addFolder,
                          createKey: const Key("library-sidebar-import"),
                          onCreate:
                              saving ||
                                  widget.isBusy ||
                                  widget.isAddingSourceDisabled
                              ? null
                              : widget.onAddSource,
                        ),
                      if (widget.roots.isEmpty &&
                          widget.transientSource == null)
                        if (widget.isCompact)
                          const AmeTooltip(
                            message: LibraryStrings.noFolder,
                            child: IconButton(
                              onPressed: null,
                              icon: Icon(Symbols.folder_off_rounded),
                            ),
                          )
                        else
                          const ListTile(
                            leading: Icon(Symbols.folder_off_rounded),
                            title: Text(LibraryStrings.noFolder),
                          ),
                    ],
                  ),
                  itemBuilder: (context, root, index, draggable) => Column(
                    children: [
                      ReorderableDragStartListener(
                        index: index,
                        enabled: draggable,
                        child: LibrarySourceNavigationTile(
                          root: root,
                          synchronizationStatus:
                              widget.rootSynchronizationStatuses[root.id],
                          hasSynchronizationFailure:
                              widget.hasSynchronizationFailure,
                          isCompact: widget.isCompact,
                          isSelected:
                              widget.selectedRootId == root.id &&
                              widget.selectedFolderRelativePath == null,
                          isExpanded: _isExpanded(root.id, ""),
                          isBrowseDisabled: widget.isBrowseDisabled,
                          isUpdating: widget.updatingRootIds.contains(root.id),
                          isUpdateDisabled:
                              widget.retainedScanRootId == root.id,
                          focusNode: _focusNodeForRoot(root.id),
                          checked: _selection.contains(root.id),
                          onToggleSelection:
                              widget.onRemoveRoots == null ||
                                  widget.isBrowseDisabled ||
                                  _removing
                              ? null
                              : () =>
                                    setState(() => _selection.toggle(root.id)),
                          onSelect: () {
                            if (_selection.isActive) {
                              setState(() => _selection.toggle(root.id));
                            } else {
                              widget.onSelectRoot(root);
                            }
                          },
                          onToggleExpansion:
                              widget.folderTree
                                  .branch(root.id, "")
                                  .folders
                                  .isNotEmpty
                              ? () => _toggleBranch(root.id, "")
                              : null,
                          onUpdate: () => widget.onUpdateRoot(root),
                          onRelocate:
                              widget.isAddingSourceDisabled ||
                                  widget.onRelocateRoot == null
                              ? null
                              : () => widget.onRelocateRoot!(root),
                          onOpen: () => widget.onOpenRoot(root),
                          onRemove: () => _requestRemoveRoot(root),
                        ),
                      ),
                      if (!widget.isCompact && _isExpanded(root.id, ""))
                        ..._buildFolderBranch(root, "", 1),
                      if (!widget.isCompact &&
                          !_isExpanded(root.id, "") &&
                          widget.folderTree.branch(root.id, "").errorMessage !=
                              null)
                        LibraryFolderErrorTile(
                          depth: 1,
                          onRetry: () =>
                              unawaited(widget.onExpandFolder(root.id, "")),
                        ),
                    ],
                  ),
                  footer: (_) => Column(
                    children: [
                      if (widget.transientSource case final source?)
                        PendingLibrarySourceTile(
                          source: source,
                          isCompact: widget.isCompact,
                        ),
                    ],
                  ),
                ),
              ),
            ],
          ),
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
      return [const LibraryFolderLoadingTile()];
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
          key: ValueKey("folder-navigation-${root.id}-${folder.relativePath}"),
          root: root,
          folder: folder,
          depth: depth,
          isSelected:
              widget.selectedRootId == root.id &&
              widget.selectedFolderRelativePath == folder.relativePath,
          isExpanded: isExpanded,
          isBusy: widget.isBrowseDisabled,
          onSelect: () => widget.onSelectFolder(root, folder),
          onToggleExpansion: folder.hasChildFolders
              ? () => _toggleBranch(root.id, folder.relativePath)
              : null,
          onOpen: () => widget.onOpenFolder(root, folder),
        ),
      );
      if (isExpanded && folder.hasChildFolders) {
        children.addAll(
          _buildFolderBranch(root, folder.relativePath, depth + 1),
        );
      }
    }
    if (branch.errorMessage != null) {
      children.add(
        LibraryFolderErrorTile(
          depth: depth,
          onRetry: () =>
              unawaited(widget.onLoadMoreFolders(root.id, parentRelativePath)),
        ),
      );
    } else if (branch.hasMore) {
      children.add(
        LoadMoreLibraryFoldersTile(
          depth: depth,
          isLoading: branch.isLoading,
          onPressed: () =>
              unawaited(widget.onLoadMoreFolders(root.id, parentRelativePath)),
        ),
      );
    } else if (branch.isLoading) {
      children.add(const LibraryFolderLoadingTile());
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

  FocusNode _focusNodeForRoot(String rootId) {
    return _rootFocusNodes.putIfAbsent(
      rootId,
      () => FocusNode(debugLabel: "Library source $rootId"),
    );
  }

  void _requestRemoveRoot(LibraryRoot root) {
    _pendingRemovalFocusRootId = root.id;
    widget.onRemoveRoot(root);
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

TextStyle? _navigationTitleStyle(BuildContext context, bool isSelected) {
  return Theme.of(context).textTheme.bodyLarge?.copyWith(
    fontWeight: isSelected ? ameFontWeightSemibold : ameFontWeightMedium,
  );
}
