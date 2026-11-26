import "dart:convert";

import "package:cedarflake_ame/app/bootstrap/diagnostic_library_snapshot.dart";
import "package:cedarflake_ame/features/diagnostics/domain/diagnostic_report.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_lifecycle.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/application/ports/library_classification.dart";
import "package:cedarflake_ame/features/library/application/ports/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../../support/application_test_bindings.dart";

void main() {
  test(
    "snapshot separates retained model and observed runtime from preferences",
    () async {
      final reads = LibraryForegroundReads();
      final lifecycle = LibraryClassificationLifecycle(
        _Inactive(),
        foregroundReads: reads,
      );
      final configuration = LibraryClassificationConfiguration(
        _Configuration(),
        lifecycle,
      );
      final container = ProviderContainer(
        overrides: applicationTestOverrides([
          libraryControllerProvider.overrideWith(_Library.new),
          libraryClassificationConfigurationProvider.overrideWithValue(
            configuration,
          ),
        ]),
      );
      try {
        final snapshot = await readDiagnosticLibrarySnapshot(container);
        final facts = {
          for (final fact in snapshot.facts) fact.field: fact.value,
        };
        expect(facts[DiagnosticField.modelState], "selected");
        expect(facts[DiagnosticField.acceleration], "gpu_preferred");
        expect(facts[DiagnosticField.classificationRuntime], "cpu");
        expect(facts[DiagnosticField.exactAnalysisPhase], "idle");
        expect(facts[DiagnosticField.similarityAnalysisPhase], "idle");
        expect(facts[DiagnosticField.rootCount], "0");
        expect(facts[DiagnosticField.scanPhase], "idle");
        expect(
          jsonEncode({
            for (final entry in facts.entries) entry.key.name: entry.value,
          }),
          isNot(contains("Private")),
        );
      } finally {
        container.dispose();
        await lifecycle.close();
        lifecycle.dispose();
        reads.dispose();
      }
    },
  );
}

class _Library extends LibraryController {
  @override
  LibraryState build() =>
      const LibraryState(catalogPath: r"C:\Private\catalog.db");
}

class _Inactive implements LibraryClassification {
  @override
  Future<LibraryClassificationSession?> open() =>
      throw StateError("Diagnostics must not start analysis");
}

class _Configuration implements LibraryClassificationConfigurationService {
  @override
  Future<LibraryClassificationAcceleration> loadAcceleration() async =>
      const LibraryClassificationAcceleration(
        gpuEnabled: true,
        deviceName: "CPU",
      );

  @override
  Future<LibraryClassificationSettings> loadNavigation() async =>
      LibraryClassificationSettings(
        revision: BigInt.one,
        dispatchRevision: BigInt.one,
        mode: LibraryClassificationMode.paused,
        categories: const [
          LibraryClassificationCategory(
            id: "PrivateId",
            label: "Private name",
            descriptions: ["Private text"],
          ),
        ],
        runId: "PrivateRun",
        installationSelected: true,
      );

  @override
  Future<LibraryClassificationSettings> load() =>
      throw UnsupportedError("Only navigation status is needed");

  @override
  Future<void> saveAcceleration({required bool gpuEnabled}) =>
      throw UnsupportedError("Diagnostics are read-only");

  @override
  Future<List<LibraryClassificationCategory>> initialCategories() =>
      throw UnsupportedError("Diagnostics do not initialize categories");

  @override
  Future<LibraryClassificationSettings> save({
    required String operationId,
    required LibraryClassificationSettings observed,
    required List<LibraryClassificationCategory> categories,
    required LibraryClassificationConfigurationAction action,
  }) => throw UnsupportedError("Diagnostics are read-only");

  @override
  Future<LibraryClassificationSettings> changeControl({
    required BigInt expectedRevision,
    required LibraryClassificationControl action,
  }) => throw UnsupportedError("Diagnostics do not control analysis");
}
