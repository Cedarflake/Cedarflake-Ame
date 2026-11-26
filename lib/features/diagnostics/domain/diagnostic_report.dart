import "dart:async";

enum DiagnosticField {
  applicationVersion,
  applicationBuild,
  operatingSystem,
  operatingSystemVersion,
  architecture,
  dartVersion,
  catalogSchemaVersion,
  storageFormatVersion,
  classificationRuntime,
  modelState,
  acceleration,
  classificationPhase,
  exactAnalysisPhase,
  similarityAnalysisPhase,
  scanPhase,
  synchronizationPhase,
  updatePhase,
  rootCount,
  availableRootCount,
  assetCount,
  activeTaskCount,
  startupState,
}

enum DiagnosticComponent {
  application,
  startup,
  catalog,
  storage,
  scan,
  synchronization,
  classification,
  exactAnalysis,
  similarityAnalysis,
  updates,
  fileOperations,
}

enum DiagnosticCodeNamespace { ame, dart, os, win32, hresult, sqlite }

class DiagnosticFact {
  DiagnosticFact(this.field, this.value) {
    if (!RegExp(r"^[A-Za-z0-9_. :+()\-]{1,160}$").hasMatch(value)) {
      throw ArgumentError(
        "Diagnostic facts must contain admitted scalar values",
      );
    }
  }

  final DiagnosticField field;
  final String value;
}

class DiagnosticNativeCode {
  DiagnosticNativeCode(this.namespace, this.value) {
    _requireToken(value);
  }

  final DiagnosticCodeNamespace namespace;
  final String value;

  String get label => "${namespace.name.toUpperCase()}:$value";
}

class DiagnosticEvent {
  DiagnosticEvent({
    required this.occurredAt,
    required this.component,
    required this.phase,
    this.code,
    this.nativeCode,
  }) {
    _requireToken(phase);
    if (code case final code?) _requireToken(code);
  }

  final DateTime occurredAt;
  final DiagnosticComponent component;
  final String phase;
  final String? code;
  final DiagnosticNativeCode? nativeCode;
}

void _requireToken(String value) {
  if (!RegExp(r"^[A-Za-z0-9_.\-]{1,80}$").hasMatch(value)) {
    throw ArgumentError("Diagnostic codes must be bounded identifiers");
  }
}

class DiagnosticSnapshot {
  DiagnosticSnapshot({
    required this.capturedAt,
    required List<DiagnosticFact> facts,
    List<DiagnosticEvent> events = const [],
    this.journalAvailable = true,
  }) : facts = List.unmodifiable(facts),
       events = List.unmodifiable(
         events.skip(events.length > 256 ? events.length - 256 : 0),
       ) {
    if (facts.length > DiagnosticField.values.length ||
        facts.map((fact) => fact.field).toSet().length != facts.length) {
      throw ArgumentError("Diagnostic fields must be unique and bounded");
    }
  }

  final DateTime capturedAt;
  final List<DiagnosticFact> facts;
  final List<DiagnosticEvent> events;
  final bool journalAvailable;
}

enum DiagnosticFailureKind { collection, destination, writing, openingFolder }

class DiagnosticFailure implements Exception {
  const DiagnosticFailure(this.kind, {this.nativeCode});
  final DiagnosticFailureKind kind;
  final DiagnosticNativeCode? nativeCode;
}

class DiagnosticCancelled implements Exception {
  const DiagnosticCancelled();
}

class DiagnosticCancellation {
  final Completer<void> _cancelled = Completer<void>();
  bool _requested = false;
  bool _committing = false;
  bool get isRequested => _requested;
  Future<void> get whenCancelled => _cancelled.future;

  void cancel() {
    if (!_committing && !_requested) {
      _requested = true;
      _cancelled.complete();
    }
  }

  void check() {
    if (_requested) throw const DiagnosticCancelled();
  }

  void beginCommit() {
    check();
    _committing = true;
  }
}
