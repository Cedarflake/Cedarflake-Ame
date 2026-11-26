import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/diagnostic_report.dart";
import "ports/diagnostic_export.dart";

final diagnosticsControllerProvider = Provider<DiagnosticsController>(
  (ref) =>
      throw StateError("Diagnostics must be bound during application startup"),
);

final diagnosticExportStateProvider = Provider<DiagnosticExportState>((ref) {
  final controller = ref.watch(diagnosticsControllerProvider);
  controller.addListener(ref.invalidateSelf);
  ref.onDispose(() => controller.removeListener(ref.invalidateSelf));
  return controller.state;
});

enum DiagnosticExportPhase {
  idle,
  choosing,
  collecting,
  writing,
  stopping,
  saved,
  cancelled,
  failed,
}

class DiagnosticExportState {
  const DiagnosticExportState({
    this.phase = DiagnosticExportPhase.idle,
    this.destination,
    this.failure,
  });

  final DiagnosticExportPhase phase;
  final String? destination;
  final DiagnosticFailure? failure;
  bool get busy => switch (phase) {
    DiagnosticExportPhase.choosing ||
    DiagnosticExportPhase.collecting ||
    DiagnosticExportPhase.writing ||
    DiagnosticExportPhase.stopping => true,
    _ => false,
  };
}

class DiagnosticsController extends ChangeNotifier {
  DiagnosticsController({required this._collector, required this._platform});

  final DiagnosticCollector _collector;
  final DiagnosticExportPlatform _platform;
  DiagnosticExportState _state = const DiagnosticExportState();
  DiagnosticExportState get state => _state;
  DiagnosticCancellation? _cancellation;
  Future<void>? _writing;
  Future<void>? _closing;
  String? _destination;
  bool _closed = false;
  int _generation = 0;

  Future<void> export(DiagnosticSaveLabels labels) async {
    if (_closed || state.busy) return;
    final generation = ++_generation;
    _destination = null;
    _publish(
      const DiagnosticExportState(phase: DiagnosticExportPhase.choosing),
    );
    try {
      final destination = await _platform.chooseDestination(labels);
      if (!_admit(generation)) return;
      if (destination == null) {
        _publish(const DiagnosticExportState());
        return;
      }
      _destination = destination;
      await _run(generation, destination);
    } on Object catch (error) {
      if (_admit(generation)) _fail(error, DiagnosticFailureKind.destination);
    }
  }

  Future<void> retry() async {
    if (_closed || state.busy) return;
    final destination = _destination;
    if (destination == null) return;
    if (state.failure?.kind == DiagnosticFailureKind.openingFolder) {
      await openFolder();
      return;
    }
    await _run(++_generation, destination);
  }

  Future<void> _run(int generation, String destination) async {
    final cancellation = DiagnosticCancellation();
    _cancellation = cancellation;
    var failureKind = DiagnosticFailureKind.collection;
    _publish(
      DiagnosticExportState(
        phase: DiagnosticExportPhase.collecting,
        destination: destination,
      ),
    );
    try {
      final snapshot = await Future.any<DiagnosticSnapshot>([
        _collector.collect().timeout(const Duration(seconds: 5)),
        cancellation.whenCancelled.then(
          (_) => throw const DiagnosticCancelled(),
        ),
      ]);
      if (!_admit(generation)) return;
      cancellation.check();
      failureKind = DiagnosticFailureKind.writing;
      _publish(
        DiagnosticExportState(
          phase: DiagnosticExportPhase.writing,
          destination: destination,
        ),
      );
      final writing = _platform.write(destination, snapshot, cancellation);
      _writing = writing;
      await writing;
      if (_admit(generation)) {
        _publish(
          DiagnosticExportState(
            phase: DiagnosticExportPhase.saved,
            destination: destination,
          ),
        );
      }
    } on DiagnosticCancelled {
      if (_admit(generation)) {
        _publish(
          const DiagnosticExportState(phase: DiagnosticExportPhase.cancelled),
        );
      }
    } on Object catch (error) {
      if (_admit(generation)) {
        if (cancellation.isRequested) {
          _publish(
            const DiagnosticExportState(phase: DiagnosticExportPhase.cancelled),
          );
        } else {
          _fail(error, failureKind);
        }
      }
    } finally {
      if (identical(_cancellation, cancellation)) {
        _cancellation = null;
        _writing = null;
      }
    }
  }

  void cancel() {
    if (!state.busy || state.phase == DiagnosticExportPhase.choosing) return;
    _cancellation?.cancel();
    _publish(
      DiagnosticExportState(
        phase: DiagnosticExportPhase.stopping,
        destination: _destination,
      ),
    );
  }

  void dismiss() {
    if (!_closed && !state.busy) {
      _generation++;
      _publish(const DiagnosticExportState());
    }
  }

  Future<void> openFolder() async {
    final destination = _destination;
    if (_closed || state.busy || destination == null) return;
    final generation = _generation;
    try {
      await _platform.openFolder(destination);
      if (_admit(generation)) {
        _publish(
          DiagnosticExportState(
            phase: DiagnosticExportPhase.saved,
            destination: destination,
          ),
        );
      }
    } on Object catch (error) {
      if (_admit(generation)) _fail(error, DiagnosticFailureKind.openingFolder);
    }
  }

  bool _admit(int generation) => !_closed && generation == _generation;

  void _fail(Object error, DiagnosticFailureKind kind) => _publish(
    DiagnosticExportState(
      phase: DiagnosticExportPhase.failed,
      destination: _destination,
      failure: error is DiagnosticFailure ? error : DiagnosticFailure(kind),
    ),
  );

  void _publish(DiagnosticExportState value) {
    if (_closed) return;
    _state = value;
    notifyListeners();
  }

  Future<void> close() => _closing ??= _close();

  Future<void> _close() async {
    _closed = true;
    _generation++;
    _cancellation?.cancel();
    try {
      await _writing;
    } on Object {
      // The export owner has already translated its terminal failure.
    }
  }
}
