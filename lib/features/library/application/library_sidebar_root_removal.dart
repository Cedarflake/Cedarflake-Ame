import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_controller.dart";
import "library_sidebar_removal.dart";

LibrarySidebarRemoval createSidebarRootRemoval({
  required List<LibraryRoot> roots,
  required LibraryController controller,
  required LibraryState Function() readState,
}) => LibrarySidebarRemoval(
  entries: [
    for (final root in roots)
      LibrarySidebarRemovalEntry(root.id, root.displayPath),
  ],
  remove: (id) async {
    final root = roots.singleWhere((root) => root.id == id);
    final prepared = controller.prepareRootRemoval(root);
    if (prepared == null) {
      return const LibrarySidebarRemovalOutcome(
        committed: false,
        error: "文件夹状态已更改或仍在更新，请稍后重试。",
      );
    }
    final removed = await controller.executeRootRemoval(prepared);
    if (removed) return const LibrarySidebarRemovalOutcome(committed: true);
    final state = readState();
    final committed = state.removingRootId == id && state.isRemovalCommitted;
    return LibrarySidebarRemovalOutcome(
      committed: committed,
      error: committed ? "文件夹已移除，图库尚未刷新。请关闭此窗口后重试图库刷新。" : "未能移除文件夹，请重试。",
    );
  },
);
