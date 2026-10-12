import "dart:async";

import "../domain/library_models.dart";
import "library_query_refresh.dart";
import "library_scan_control.dart";
import "library_scan_source_admission.dart";
import "library_scanner.dart";

abstract interface class LibraryScanRunListener {
  void onScanUpdate(LibraryScanRun run, LibraryScanUpdate update);
  void onScanError(LibraryScanRun run, Object error);
  void onScanDone(LibraryScanRun run);
}

/// One stream identity survives its terminal event until native work drains.
class LibraryScanRun {
  LibraryScanRun({
    required this.scanId,
    required this.generation,
    required this.sourceAdmission,
    required this.queryIntent,
    required LibraryScanner scanner,
  }) : control = LibraryScanControl(scanId: scanId, scanner: scanner);

  final String scanId;
  final int generation;
  final LibraryScanSourceAdmission sourceAdmission;
  final LibraryQueryIntent queryIntent;
  final LibraryScanControl control;
  final Completer<void> _streamDone = Completer<void>();
  final Completer<void> _retired = Completer<void>();
  StreamSubscription<LibraryScanUpdate>? _subscription;
  bool didStart = false;
  bool didReceiveTerminal = false;
  bool _hasProtocolFailure = false;
  LibraryScanAlreadyCovered? _coveredFolder;

  LibraryScanAlreadyCovered? get coveredFolder => _coveredFolder;

  Future<void> get streamDone => _streamDone.future;
  Future<void> get retired => _retired.future;
  bool get isDone => _streamDone.isCompleted;
  bool get hasProtocolFailure => _hasProtocolFailure;

  void listen(
    Stream<LibraryScanUpdate> stream,
    LibraryScanRunListener listener,
  ) {
    _subscription = stream.listen(
      (update) {
        if (isDone) {
          return;
        }
        if (didReceiveTerminal) {
          if (_hasProtocolFailure &&
              update is LibraryScanStarted &&
              update.scanId == scanId) {
            control.started();
          }
          return;
        }
        final eventScanId = switch (update) {
          LibraryScanStarted(:final scanId) ||
          LibraryScanAlreadyCovered(:final scanId) => scanId,
          _ => null,
        };
        if (eventScanId != null && eventScanId != scanId) {
          _rejectProtocol(
            listener,
            const LibraryScanFailure(
              code: "bridge_scan_id_mismatch",
              message: "Received an event for a different scan",
            ),
          );
          return;
        }
        if (update is LibraryScanAlreadyCovered) {
          if (didStart) {
            _rejectProtocol(
              listener,
              const LibraryScanFailure(
                code: "bridge_scan_coverage_after_start",
                message:
                    "A registered scan cannot finish as an existing folder selection",
              ),
            );
            return;
          }
          if (control.command == LibraryScanCommand.cancel ||
              control.command == LibraryScanCommand.suspend) {
            update = const LibraryScanCancelled(
              acceptedItems: 0,
              issueCount: 0,
            );
          } else {
            _coveredFolder = update;
          }
        }
        if (update is LibraryScanStarted) {
          if (didStart) {
            return;
          }
          didStart = true;
          control.started();
        }
        didReceiveTerminal =
            didReceiveTerminal ||
            switch (update) {
              LibraryScanCompleted() ||
              LibraryScanCancelled() ||
              LibraryScanPaused() ||
              LibraryScanStale() ||
              LibraryScanAlreadyCovered() ||
              LibraryScanFailed() => true,
              _ => false,
            };
        if (didReceiveTerminal) {
          control.retire();
        }
        listener.onScanUpdate(this, update);
      },
      onError: (Object error, StackTrace stackTrace) {
        if (didReceiveTerminal || isDone) {
          return;
        }
        didReceiveTerminal = true;
        _hasProtocolFailure = true;
        control.request(LibraryScanCommand.cancel);
        listener.onScanError(this, error);
      },
      onDone: () {
        complete();
        listener.onScanDone(this);
      },
      cancelOnError: false,
    );
  }

  void complete() {
    control.retire();
    if (!_streamDone.isCompleted) {
      _streamDone.complete();
    }
  }

  void _rejectProtocol(
    LibraryScanRunListener listener,
    LibraryScanFailure failure,
  ) {
    didReceiveTerminal = true;
    _hasProtocolFailure = true;
    control.request(LibraryScanCommand.cancel);
    listener.onScanError(this, failure);
  }

  Future<void> dispose() async {
    control.retire();
    await _subscription?.cancel();
    _subscription = null;
    complete();
    if (!_retired.isCompleted) {
      _retired.complete();
    }
  }
}
