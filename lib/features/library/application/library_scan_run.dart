import "dart:async";

import "../domain/library_models.dart";

abstract interface class LibraryScanRunListener {
  void onScanUpdate(LibraryScanRun run, LibraryScanUpdate update);
  void onScanError(LibraryScanRun run, Object error);
  void onScanDone(LibraryScanRun run);
}

/// One stream identity survives its terminal event until native work drains.
class LibraryScanRun {
  LibraryScanRun({required this.scanId, required this.generation});

  final String scanId;
  final int generation;
  final Completer<void> _streamDone = Completer<void>();
  StreamSubscription<LibraryScanUpdate>? _subscription;
  bool didStart = false;
  bool didReceiveTerminal = false;

  Future<void> get streamDone => _streamDone.future;
  bool get isDone => _streamDone.isCompleted;

  void listen(
    Stream<LibraryScanUpdate> stream,
    LibraryScanRunListener listener,
  ) {
    _subscription = stream.listen(
      (update) {
        if (update case LibraryScanStarted(:final scanId)) {
          if (scanId != this.scanId) {
            listener.onScanError(
              this,
              const LibraryScanFailure(
                code: "bridge_scan_id_mismatch",
                message: "Received a start event for a different scan",
              ),
            );
            return;
          }
          didStart = true;
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
        listener.onScanUpdate(this, update);
      },
      onError: (Object error, StackTrace stackTrace) {
        complete();
        listener.onScanError(this, error);
      },
      onDone: () {
        complete();
        listener.onScanDone(this);
      },
      cancelOnError: true,
    );
  }

  void complete() {
    if (!_streamDone.isCompleted) {
      _streamDone.complete();
    }
  }

  Future<void> dispose() async {
    await _subscription?.cancel();
    _subscription = null;
    complete();
  }
}
