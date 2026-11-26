import "../../domain/diagnostic_report.dart";

abstract interface class DiagnosticCollector {
  Future<DiagnosticSnapshot> collect();
}

class DiagnosticSaveLabels {
  const DiagnosticSaveLabels({required this.type, required this.save});
  final String type;
  final String save;
}

abstract interface class DiagnosticExportPlatform {
  Future<String?> chooseDestination(DiagnosticSaveLabels labels);
  Future<void> write(
    String destination,
    DiagnosticSnapshot snapshot,
    DiagnosticCancellation cancellation,
  );
  Future<void> openFolder(String destination);
}
