import "../application/library_update_controller.dart";
import "../domain/library_state.dart";

/// Independent retained intent must not hide another root's actionable work.
class LibraryTaskSurfaceSelection {
  LibraryTaskSurfaceSelection(LibraryState state, LibraryUpdateState updates) {
    final primaryStatus = state.taskKind == LibraryTaskKind.remove
        ? state.status
        : state.primaryScanSnapshot.status;
    hasPrimary = switch (primaryStatus) {
      LibraryStatus.empty => false,
      LibraryStatus.completed => state.scanId != null,
      _ => true,
    };
    final ownsPrimarySurface =
        hasPrimary &&
        (state.isCommittedRemovalReloadPending ||
            switch (primaryStatus) {
              LibraryStatus.choosingDirectory ||
              LibraryStatus.scanning ||
              LibraryStatus.pausing ||
              LibraryStatus.cancelling ||
              LibraryStatus.discarding ||
              LibraryStatus.removing ||
              LibraryStatus.refreshing ||
              LibraryStatus.paused => true,
              _ => false,
            });
    final hasRetryablePrimary =
        primaryStatus == LibraryStatus.failed ||
        primaryStatus == LibraryStatus.stale ||
        primaryStatus == LibraryStatus.cancelled;
    showPrimary =
        hasPrimary &&
        (!updates.hasFeedback || ownsPrimarySurface || hasRetryablePrimary);
    showUpdates =
        updates.hasFeedback &&
        (!ownsPrimarySurface ||
            state.hasRetainedScan ||
            state.primaryScanSnapshot.publication ==
                LibraryScanPublication.reloadPending ||
            (primaryStatus == LibraryStatus.removing && updates.hasActive));
  }

  late final bool hasPrimary;
  late final bool showPrimary;
  late final bool showUpdates;
}
