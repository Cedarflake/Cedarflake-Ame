import "../../../app/messages/ame_message.dart";
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
        error: AmeLabel.folderStillUpdating,
      );
    }
    final removed = await controller.executeRootRemoval(prepared);
    if (removed) return const LibrarySidebarRemovalOutcome(committed: true);
    final state = readState();
    final committed = state.removingRootId == id && state.isRemovalCommitted;
    return LibrarySidebarRemovalOutcome(
      committed: committed,
      error: committed
          ? AmeLabel.folderRemovedRefreshPending
          : AmeLabel.folderRemovalFailed,
    );
  },
);
