import "../domain/library_state.dart";

bool canEditLibrarySearch(LibraryState state) {
  if (!state.isRefreshingQuery) {
    return !state.isBusy;
  }
  if (state.isLoadingTimeAnchor || state.isCommittedRemovalReloadPending) {
    return false;
  }
  if (state.status == LibraryStatus.removing) {
    return false;
  }
  // Query loading may be superseded; primary task execution keeps its own gate.
  return !state.primaryScanSnapshot.blocksExecution;
}
