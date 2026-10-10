import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_scan_session.dart";

typedef LibraryScanCheckpointLoader =
    Future<RecoverableLibraryScan?> Function();

class LibraryScanRestoration {
  LibraryScanRestoration({
    required this._loadRecoverable,
    required this._loadPaused,
    required this._session,
    required this._readState,
    required this._writeState,
    required this._readScanSequence,
    required this._isUnavailable,
  });

  final LibraryScanCheckpointLoader _loadRecoverable;
  final LibraryScanCheckpointLoader _loadPaused;
  final LibraryScanSession _session;
  final LibraryPrimaryScanSnapshot Function() _readState;
  final void Function(LibraryPrimaryScanSnapshot) _writeState;
  final int Function() _readScanSequence;
  final bool Function() _isUnavailable;

  Future<void> restore() async {
    final sequence = _readScanSequence();
    if (!_canRestore(sequence)) {
      return;
    }
    try {
      final recoverable = await _loadRecoverable();
      if (!_canRestore(sequence)) {
        return;
      }
      final checkpoint = recoverable ?? await _loadPaused();
      if (checkpoint == null || !_canRestore(sequence)) {
        return;
      }
      _session.restorePaused(checkpoint);
      // Catalog recovery exposes only unpublished foreground first imports.
      // Loading a checkpoint restores intent; only explicit Continue executes it.
      _writeState(
        _readState().copyWith(
          status: LibraryStatus.paused,
          scanId: checkpoint.scanId,
          rootPath: checkpoint.rootPath,
          displayRootPath: checkpoint.displayRootPath,
          taskKind: LibraryTaskKind.import,
          visitedEntries: checkpoint.visitedEntries,
          stagedAssetCount: checkpoint.acceptedItems,
          issueCount: checkpoint.issueCount,
          itemLimit: checkpoint.itemLimit,
          entryLimit: checkpoint.entryLimit,
          isResumingScan: false,
          errorMessage: null,
        ),
      );
    } on Object catch (error) {
      if (_canRestore(sequence)) {
        _writeState(
          _readState().copyWith(
            status: LibraryStatus.failed,
            taskKind: LibraryTaskKind.import,
            errorMessage: error.toString(),
          ),
        );
      }
    }
  }

  bool _canRestore(int sequence) {
    if (_isUnavailable() || _readScanSequence() != sequence) {
      return false;
    }
    final state = _readState();
    return !state.blocksExecution &&
        (state.status == LibraryStatus.empty ||
            state.status == LibraryStatus.completed);
  }
}
