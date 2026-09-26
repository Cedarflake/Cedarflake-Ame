import "dart:async";

import "../domain/library_models.dart";
import "library_scan_control.dart";
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
    required LibraryScanner scanner,
  }) : control = LibraryScanControl(scanId: scanId, scanner: scanner);

  final String scanId;
  final int generation;
  final LibraryScanControl control;
  final Completer<void> _streamDone = Completer<void>();
  StreamSubscription<LibraryScanUpdate>? _subscription;
  bool didStart = false;
  bool didReceiveTerminal = false;
  bool _hasProtocolFailure = false;

  Future<void> get streamDone => _streamDone.future;
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
        if (update case LibraryScanStarted(:final scanId)) {
          if (scanId != this.scanId) {
            didReceiveTerminal = true;
            _hasProtocolFailure = true;
            control.request(LibraryScanCommand.cancel);
            listener.onScanError(
              this,
              const LibraryScanFailure(
                code: "bridge_scan_id_mismatch",
                message: "Received a start event for a different scan",
              ),
            );
            return;
          }
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

  Future<void> dispose() async {
    control.retire();
    await _subscription?.cancel();
    _subscription = null;
    complete();
  }
}
