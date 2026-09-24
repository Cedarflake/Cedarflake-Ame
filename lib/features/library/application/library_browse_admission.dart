import "../domain/library_state.dart";

/// Reads use the published catalog independently of an active scan's staging.
/// Publication leases and result generations remain with the viewport owner.
class LibraryBrowseAdmission {
  const LibraryBrowseAdmission(this._state);

  final LibraryState _state;

  bool get _isUnpublishedScan =>
      _state.isScanning &&
      !_state.roots.any((root) => root.activeScanId != null);

  bool get _isTaskBlocking => switch (_state.status) {
    LibraryStatus.choosingDirectory ||
    LibraryStatus.removing ||
    LibraryStatus.refreshing => true,
    LibraryStatus.scanning ||
    LibraryStatus.pausing ||
    LibraryStatus.cancelling => _isUnpublishedScan,
    _ => false,
  };

  bool get canBrowse =>
      !_isTaskBlocking &&
      !_state.isRefreshingQuery &&
      !_state.isLoadingTimeAnchor &&
      !_state.isCommittedRemovalReloadPending;

  bool get canReadPage =>
      canBrowse && !_state.isLoadingPage && !_state.isLoadingPreviousPage;

  bool get canNavigateTime =>
      !_isTaskBlocking &&
      !_state.isRefreshingQuery &&
      !_state.isLoadingPage &&
      !_state.isLoadingPreviousPage &&
      !_state.isLoadingTimeAnchor;

  bool canStartQuery({required bool hasVisibleRangeRequest}) {
    if (_state.status == LibraryStatus.choosingDirectory ||
        _isUnpublishedScan) {
      return false;
    }
    if (_state.isLoadingTimeAnchor && !hasVisibleRangeRequest) {
      return false;
    }
    return true;
  }
}
