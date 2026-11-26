import "dart:convert";

import "package:archive/archive.dart";

import "../../domain/diagnostic_report.dart";

const diagnosticByteLimit = 128 * 1024;

Map<String, Object?> encodeDiagnosticEvent(DiagnosticEvent event) => {
  "time": event.occurredAt.toUtc().toIso8601String(),
  "component": event.component.name,
  "phase": event.phase,
  if (event.code != null) "code": event.code,
  if (event.nativeCode case final code?)
    "native": {"namespace": code.namespace.name, "value": code.value},
};

DiagnosticEvent decodeDiagnosticEvent(Object? value) {
  if (value is! Map<String, Object?>) {
    throw const FormatException("Invalid diagnostic event");
  }
  final Object? native = value["native"];
  if (native != null && native is! Map<String, Object?>) {
    throw const FormatException("Invalid diagnostic code");
  }
  final nativeMap = native as Map<String, Object?>?;
  return DiagnosticEvent(
    occurredAt: DateTime.parse(value["time"] as String),
    component: DiagnosticComponent.values.byName(value["component"] as String),
    phase: value["phase"] as String,
    code: value["code"] as String?,
    nativeCode: nativeMap == null
        ? null
        : DiagnosticNativeCode(
            DiagnosticCodeNamespace.values.byName(
              nativeMap["namespace"] as String,
            ),
            nativeMap["value"] as String,
          ),
  );
}

List<int> encodeDiagnosticArchive(DiagnosticSnapshot snapshot) {
  final report = const JsonEncoder.withIndent("  ").convert({
    "format": 1,
    "capturedAt": snapshot.capturedAt.toUtc().toIso8601String(),
    "facts": {for (final fact in snapshot.facts) fact.field.name: fact.value},
    "journalAvailable": snapshot.journalAvailable,
    "events": snapshot.events.map(encodeDiagnosticEvent).toList(),
  });
  final summary = StringBuffer("Ame diagnostic report\n")
    ..writeln("Captured: ${snapshot.capturedAt.toUtc().toIso8601String()}")
    ..writeln("Format: 1")
    ..writeln(
      "Personal paths, media, catalog records and credentials are excluded.",
    )
    ..writeln();
  for (final fact in snapshot.facts) {
    summary.writeln("${fact.field.name}: ${fact.value}");
  }
  summary
    ..writeln()
    ..writeln("Recent events (${snapshot.events.length})")
    ..writeln(
      "Persistent journal: ${snapshot.journalAvailable ? 'available' : 'unavailable'}",
    );
  for (final event in snapshot.events) {
    summary.writeln(
      [
        event.occurredAt.toUtc().toIso8601String(),
        event.component.name,
        event.phase,
        ?event.code,
        if (event.nativeCode case final code?) code.label,
      ].join(" | "),
    );
  }
  final summaryBytes = utf8.encode(summary.toString());
  final reportBytes = utf8.encode(report);
  if (summaryBytes.length + reportBytes.length > diagnosticByteLimit * 2) {
    throw const DiagnosticFailure(DiagnosticFailureKind.collection);
  }
  final archive = Archive()
    ..addFile(ArchiveFile("summary.txt", summaryBytes.length, summaryBytes))
    ..addFile(ArchiveFile("diagnostics.json", reportBytes.length, reportBytes));
  return ZipEncoder().encode(archive);
}
