import "dart:async";

import "package:cedarflake_ame/features/diagnostics/application/diagnostics_controller.dart";
import "package:cedarflake_ame/features/diagnostics/application/ports/diagnostic_export.dart";
import "package:cedarflake_ame/features/diagnostics/domain/diagnostic_report.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

const labels = DiagnosticSaveLabels(type: "Diagnostics", save: "Save");

void main() {
  test("cancelling the native picker collects no diagnostic data", () async {
    final source = _Collector();
    final platform = _Platform()..destination = null;
    final controller = DiagnosticsController(
      collector: source,
      platform: platform,
    );
    await controller.export(labels);
    expect(source.calls, 0);
    expect(platform.writes, 0);
    expect(controller.state.phase, DiagnosticExportPhase.idle);
    await controller.close();
    controller.dispose();
  });

  test(
    "cancel during collection never writes the selected destination",
    () async {
      final source = _Collector()..pending = Completer<DiagnosticSnapshot>();
      final platform = _Platform();
      final controller = DiagnosticsController(
        collector: source,
        platform: platform,
      );
      final exporting = controller.export(labels);
      await source.started.future;
      controller.dismiss();
      expect(controller.state.phase, DiagnosticExportPhase.collecting);
      controller.cancel();
      expect(controller.state.phase, DiagnosticExportPhase.stopping);
      await exporting;
      expect(source.pending!.isCompleted, isFalse);
      expect(platform.writes, 0);
      expect(controller.state.phase, DiagnosticExportPhase.cancelled);
      source.pending!.complete(_snapshot());
      await controller.close();
      controller.dispose();
    },
  );

  test(
    "write failure retains its native code and retry re-collects current state",
    () async {
      final source = _Collector();
      final platform = _Platform()
        ..failure = DiagnosticFailure(
          DiagnosticFailureKind.writing,
          nativeCode: DiagnosticNativeCode(DiagnosticCodeNamespace.os, "32"),
        );
      final controller = DiagnosticsController(
        collector: source,
        platform: platform,
      );
      await controller.export(labels);
      expect(controller.state.failure?.nativeCode?.label, "OS:32");
      expect(controller.state.phase, DiagnosticExportPhase.failed);
      platform.failure = null;
      await controller.retry();
      expect(source.calls, 2);
      expect(platform.picks, 1);
      expect(controller.state.phase, DiagnosticExportPhase.saved);
      await controller.openFolder();
      expect(platform.opened, "report.zip");
      controller.dismiss();
      expect(controller.state.phase, DiagnosticExportPhase.idle);
      await controller.close();
      controller.dispose();
    },
  );

  test(
    "shutdown discards late collection without waiting for unrelated work",
    () async {
      final source = _Collector()..pending = Completer<DiagnosticSnapshot>();
      final platform = _Platform();
      final controller = DiagnosticsController(
        collector: source,
        platform: platform,
      );
      final exporting = controller.export(labels);
      await source.started.future;
      await controller.close();
      controller.dispose();
      source.pending!.complete(_snapshot());
      await exporting;
      expect(platform.writes, 0);
    },
  );

  test("shutdown joins the cancelled writer before returning", () async {
    final source = _Collector();
    final platform = _Platform()..writeRelease = Completer<void>();
    final controller = DiagnosticsController(
      collector: source,
      platform: platform,
    );
    final exporting = controller.export(labels);
    await platform.writeStarted.future;
    var closed = false;
    final closing = controller.close().then((_) => closed = true);
    await Future<void>.delayed(Duration.zero);
    expect(closed, isFalse);
    expect(platform.cancellation?.isRequested, isTrue);
    platform.writeRelease!.complete();
    await exporting;
    await closing;
    expect(closed, isTrue);
    controller.dispose();
  });

  test(
    "fallback container retains the shared controller and observes updates",
    () async {
      final controller = DiagnosticsController(
        collector: _Collector(),
        platform: _Platform(),
      );
      ProviderContainer container() => ProviderContainer(
        overrides: [
          diagnosticsControllerProvider.overrideWithValue(controller),
        ],
      );
      final failedAssembly = container();
      expect(
        failedAssembly.read(diagnosticExportStateProvider).phase,
        DiagnosticExportPhase.idle,
      );
      failedAssembly.dispose();
      final fallback = container();
      final phases = <DiagnosticExportPhase>[];
      final subscription = fallback.listen(
        diagnosticExportStateProvider,
        (_, next) => phases.add(next.phase),
        fireImmediately: true,
      );
      await controller.export(labels);
      expect(
        fallback.read(diagnosticExportStateProvider).phase,
        DiagnosticExportPhase.saved,
      );
      expect(phases, contains(DiagnosticExportPhase.saved));
      subscription.close();
      fallback.dispose();
      await controller.close();
      controller.dispose();
    },
  );

  test("opening folder retry clears only the opening failure", () async {
    final source = _Collector();
    final platform = _Platform();
    final controller = DiagnosticsController(
      collector: source,
      platform: platform,
    );
    await controller.export(labels);
    platform.openFailure = const DiagnosticFailure(
      DiagnosticFailureKind.openingFolder,
    );
    await controller.openFolder();
    expect(controller.state.failure?.kind, DiagnosticFailureKind.openingFolder);
    platform.openFailure = null;
    await controller.retry();
    expect(controller.state.phase, DiagnosticExportPhase.saved);
    expect(controller.state.destination, "report.zip");
    expect(controller.state.failure, isNull);
    expect(source.calls, 1);
    expect(platform.writes, 1);
    await controller.close();
    controller.dispose();
  });
}

DiagnosticSnapshot _snapshot() => DiagnosticSnapshot(
  capturedAt: DateTime.utc(2026, 11, 26),
  facts: [DiagnosticFact(DiagnosticField.applicationVersion, "0.1.0-beta.1")],
);

class _Collector implements DiagnosticCollector {
  int calls = 0;
  final started = Completer<void>();
  Completer<DiagnosticSnapshot>? pending;

  @override
  Future<DiagnosticSnapshot> collect() async {
    calls++;
    if (!started.isCompleted) started.complete();
    final waiting = pending;
    if (waiting != null) return waiting.future;
    return _snapshot();
  }
}

class _Platform implements DiagnosticExportPlatform {
  String? destination = "report.zip";
  int picks = 0;
  int writes = 0;
  DiagnosticFailure? failure;
  DiagnosticFailure? openFailure;
  String? opened;
  DiagnosticCancellation? cancellation;
  final writeStarted = Completer<void>();
  Completer<void>? writeRelease;

  @override
  Future<String?> chooseDestination(DiagnosticSaveLabels labels) async {
    picks++;
    return destination;
  }

  @override
  Future<void> write(
    String destination,
    DiagnosticSnapshot snapshot,
    DiagnosticCancellation cancellation,
  ) async {
    writes++;
    this.cancellation = cancellation;
    if (!writeStarted.isCompleted) writeStarted.complete();
    if (failure case final error?) throw error;
    await writeRelease?.future;
    cancellation.check();
    cancellation.beginCommit();
  }

  @override
  Future<void> openFolder(String destination) async {
    if (openFailure case final error?) throw error;
    opened = destination;
  }
}
