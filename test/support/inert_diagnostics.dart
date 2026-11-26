import "package:cedarflake_ame/features/diagnostics/application/diagnostics_controller.dart";
import "package:cedarflake_ame/features/diagnostics/application/ports/diagnostic_export.dart";
import "package:cedarflake_ame/features/diagnostics/domain/diagnostic_report.dart";
import "package:flutter_riverpod/misc.dart";

Override inertDiagnostics() =>
    diagnosticsControllerProvider.overrideWith((ref) {
      final fixture = _Diagnostics();
      final controller = DiagnosticsController(
        collector: fixture,
        platform: fixture,
      );
      ref.onDispose(controller.dispose);
      return controller;
    });

class _Diagnostics implements DiagnosticCollector, DiagnosticExportPlatform {
  @override
  Future<DiagnosticSnapshot> collect() async =>
      DiagnosticSnapshot(capturedAt: DateTime.utc(2026), facts: []);

  @override
  Future<String?> chooseDestination(DiagnosticSaveLabels labels) async => null;

  @override
  Future<void> openFolder(String destination) =>
      throw StateError("Opening folders is not part of this fixture");

  @override
  Future<void> write(
    String destination,
    DiagnosticSnapshot snapshot,
    DiagnosticCancellation cancellation,
  ) => throw StateError("Writing diagnostics is not part of this fixture");
}
