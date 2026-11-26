import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../features/diagnostics/domain/diagnostic_report.dart";
import "../../features/library/application/library_classification_configuration.dart";
import "../../features/library/application/library_classification_lifecycle.dart";
import "../../features/library/application/library_controller.dart";
import "../../features/library/application/library_file_operations.dart";
import "../../features/library/application/library_exact_analysis_controller.dart";
import "../../features/library/application/library_similar_analysis_controller.dart";
import "../../features/library/application/library_model_installation.dart";
import "../../features/library/application/library_synchronization.dart";
import "../../features/library/domain/library_models.dart";
import "../../features/library/domain/library_state.dart";
import "../../features/updates/application/application_update_controller.dart";
import "diagnostic_error_projection.dart";

Future<({List<DiagnosticFact> facts, List<DiagnosticEvent> events})>
readDiagnosticLibrarySnapshot(ProviderContainer container) async {
  final state = container.read(libraryControllerProvider);
  final sync = container.read(librarySynchronizationProvider).current;
  final files = container.read(libraryFileActionsProvider);
  final model = container.read(libraryModelInstallationProvider);
  final updates = container.read(applicationUpdateControllerProvider).state;
  final classification = container.read(libraryClassificationLifecycleProvider);
  final exact = container.read(libraryExactAnalysisControllerProvider);
  final similar = container.read(librarySimilarAnalysisControllerProvider);
  final now = DateTime.now().toUtc();
  final facts = [
    DiagnosticFact(DiagnosticField.rootCount, "${state.roots.length}"),
    DiagnosticFact(
      DiagnosticField.availableRootCount,
      "${state.roots.where((root) => root.availability == LibraryRootAvailability.available).length}",
    ),
    DiagnosticFact(DiagnosticField.scanPhase, switch (state.primaryScan) {
      LibraryPrimaryScanSnapshot(
        status: LibraryStatus.scanning,
        :final scanPhase,
      ) =>
        scanPhase.name,
      LibraryPrimaryScanSnapshot(status: LibraryStatus.empty) || null => "idle",
      LibraryPrimaryScanSnapshot(:final status) => status.name,
    }),
    DiagnosticFact(
      DiagnosticField.synchronizationPhase,
      sync.isRunning ? "running" : "stopped",
    ),
    DiagnosticFact(DiagnosticField.updatePhase, updates.phase.name),
    DiagnosticFact(DiagnosticField.exactAnalysisPhase, switch ((
      exact.hasFailure,
      exact.isStopping,
      exact.progress?.phase.name,
    )) {
      (true, _, _) => "failed",
      (_, true, _) => "stopping",
      (_, _, final phase?) => phase,
      _ => "idle",
    }),
    DiagnosticFact(DiagnosticField.similarityAnalysisPhase, switch ((
      similar.hasFailure,
      similar.progress?.phase.name,
    )) {
      (true, _) => "failed",
      (_, final phase?) => phase,
      _ => "idle",
    }),
  ];
  final events = <DiagnosticEvent>[
    if (model != null)
      DiagnosticEvent(
        occurredAt: now,
        component: DiagnosticComponent.classification,
        phase: "installation_${model.stage.name}",
      ),
    for (final issue in state.recentIssues.take(32))
      DiagnosticEvent(
        occurredAt: now,
        component: DiagnosticComponent.scan,
        phase: "issue",
        code: diagnosticCode(issue.code),
      ),
    for (final root in sync.roots.values.take(32))
      DiagnosticEvent(
        occurredAt: root.phaseStartedAt,
        component: DiagnosticComponent.synchronization,
        phase: root.phase.name,
        code: diagnosticCode(root.lastIssueCode),
      ),
    if (files.progress case final progress?)
      DiagnosticEvent(
        occurredAt: now,
        component: DiagnosticComponent.fileOperations,
        phase: progress.phase.name,
      ),
    if (files.progress?.failure case final failure?)
      diagnosticErrorEvent(
        failure,
        component: DiagnosticComponent.fileOperations,
      ),
    if (model?.error case final failure?)
      diagnosticErrorEvent(
        failure,
        component: DiagnosticComponent.classification,
      ),
  ];
  if (classification?.error case final failure?) {
    events.add(
      diagnosticErrorEvent(
        failure,
        component: DiagnosticComponent.classification,
      ),
    );
  }
  if (updates.failure case final failure?) {
    events.add(
      diagnosticErrorEvent(failure, component: DiagnosticComponent.updates),
    );
  }
  if (similar.error case final failure?) {
    events.add(
      diagnosticErrorEvent(
        failure,
        component: DiagnosticComponent.similarityAnalysis,
      ),
    );
  }
  final configuration = container.read(
    libraryClassificationConfigurationProvider,
  );
  var modelState = "unavailable";
  if (configuration != null) {
    try {
      final acceleration = await configuration.loadAcceleration().timeout(
        const Duration(seconds: 1),
      );
      facts.add(
        DiagnosticFact(
          DiagnosticField.acceleration,
          acceleration.gpuEnabled ? "gpu_preferred" : "cpu",
        ),
      );
      final device = acceleration.deviceName;
      facts.add(
        DiagnosticFact(DiagnosticField.classificationRuntime, switch (device) {
          null => "not_initialized",
          "CPU" => "cpu",
          _ => "gpu",
        }),
      );
      final settings = await configuration.loadNavigation().timeout(
        const Duration(seconds: 1),
      );
      modelState = settings.installationSelected ? "selected" : "not_selected";
      facts.add(
        DiagnosticFact(DiagnosticField.classificationPhase, settings.mode.name),
      );
    } on Object catch (error) {
      events.add(
        diagnosticErrorEvent(
          error,
          component: DiagnosticComponent.classification,
        ),
      );
    }
  }
  facts.add(DiagnosticFact(DiagnosticField.modelState, modelState));
  return (facts: facts, events: events);
}

String? diagnosticCode(String? value) {
  if (value == null) return null;
  return RegExp(r'^[A-Za-z0-9_.-]{1,80}$').hasMatch(value)
      ? value
      : "unclassified_error";
}
