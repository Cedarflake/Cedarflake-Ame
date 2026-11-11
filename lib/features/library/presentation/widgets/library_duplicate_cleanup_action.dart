import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../application/library_file_operations.dart";
import "../../domain/library_file_operations.dart";
import "../coordination/library_duplicate_workspace.dart";
import "library_file_action_dialog.dart";

class LibraryDuplicateCleanupAction extends ConsumerStatefulWidget {
  const LibraryDuplicateCleanupAction({required this.workspace, super.key});
  final LibraryDuplicateWorkspace workspace;

  @override
  ConsumerState<LibraryDuplicateCleanupAction> createState() =>
      _LibraryDuplicateCleanupActionState();
}

class _LibraryDuplicateCleanupActionState
    extends ConsumerState<LibraryDuplicateCleanupAction> {
  bool _opening = false;

  Future<void> _open() async {
    final workspace = widget.workspace;
    final selectionRevision = workspace.cleanupSelection.revision;
    final similar = workspace.hasMembers
        ? workspace.isSimilar
        : workspace.view == LibraryDuplicateView.similar;
    final List<LibraryCleanupGroup> groups;
    if (!workspace.hasMembers) {
      groups = workspace.cleanupSelection.groups;
    } else if (similar) {
      final group = workspace.similar.selected;
      if (group == null) return;
      groups = [LibrarySimilarCleanupGroup(group.id)];
    } else {
      final group = workspace.exact.selected;
      if (group == null) return;
      groups = [
        LibraryExactCleanupGroup(group.group.digest, group.group.byteLength),
      ];
    }
    setState(() => _opening = true);
    try {
      final submission = await showLibraryFileAction(
        context: context,
        ref: ref,
        selection: LibraryDuplicateCleanup(
          kind: similar ? LibraryCleanupKind.similar : LibraryCleanupKind.exact,
          degree: workspace.similar.effectiveDegree,
          allGroups: !workspace.hasMembers && groups.isEmpty,
          groups: groups,
        ),
        action: LibraryFileAction.recycle,
      );
      final result = await submission?.completion;
      if (mounted &&
          (result?.published ?? 0) > 0 &&
          workspace.cleanupSelection.revision == selectionRevision) {
        workspace.cleanupSelection.clear();
      }
    } finally {
      if (mounted) setState(() => _opening = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final workspace = widget.workspace;
    if (workspace.view == LibraryDuplicateView.ignored ||
        (workspace.hasMembers && workspace.ignored)) {
      return const SizedBox.shrink();
    }
    final selected =
        !workspace.hasMembers && !workspace.cleanupSelection.isEmpty;
    final enabled =
        !_opening &&
        !workspace.loading &&
        !workspace.saving &&
        !workspace.loadFailed &&
        workspace.itemCount > 0 &&
        !ref.watch(libraryFileActionsProvider).busy;
    final similar = workspace.hasMembers
        ? workspace.isSimilar
        : workspace.view == LibraryDuplicateView.similar;
    final kind = similar ? "相似" : "重复";
    final label = "删除$kind图片";
    return Wrap(
      alignment: WrapAlignment.end,
      crossAxisAlignment: WrapCrossAlignment.center,
      spacing: 4,
      children: [
        if (selected)
          TextButton.icon(
            onPressed: _opening ? null : workspace.cleanupSelection.clear,
            icon: const Icon(Symbols.close_rounded),
            label: const Text("取消选择"),
          ),
        TextButton.icon(
          key: const Key("duplicate-cleanup"),
          onPressed: enabled ? () => unawaited(_open()) : null,
          icon: const Icon(Symbols.delete_rounded),
          label: Text(label),
        ),
      ],
    );
  }
}
