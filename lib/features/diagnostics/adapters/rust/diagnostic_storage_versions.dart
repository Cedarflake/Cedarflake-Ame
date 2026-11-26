import "../../../../src/rust/api/diagnostics.dart" as native;

import "../../domain/diagnostic_report.dart";

List<DiagnosticFact> diagnosticStorageFacts() {
  final versions = native.diagnosticStorageVersions();
  return [
    DiagnosticFact(DiagnosticField.catalogSchemaVersion, "${versions.catalog}"),
    DiagnosticFact(
      DiagnosticField.storageFormatVersion,
      "${versions.settings}",
    ),
  ];
}
