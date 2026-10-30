import "dart:convert";
import "dart:io";

import "package:cedarflake_ame/features/library/adapters/rust_library_classification.dart";
import "package:cedarflake_ame/features/library/adapters/rust_library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_lifecycle.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:flutter/foundation.dart";
import "package:flutter_test/flutter_test.dart";

String classificationEnvironment(String name) {
  final value = Platform.environment["CEDARFLAKE_AME_CLASSIFICATION_$name"];
  if (value == null || value.isEmpty) {
    throw StateError(
      "Run this test through integration_test_classification.ps1",
    );
  }
  return value;
}

class ClassificationObservation {
  ClassificationObservation(Map<String, dynamic> value)
    : predictions = Map<String, int>.from(value["predictions"] as Map),
      work = Map<String, int>.from(value["work"] as Map),
      features = Map<String, String>.from(value["features"] as Map),
      issues = List<String>.from(value["issues"] as List);

  final Map<String, int> predictions;
  final Map<String, int> work;
  final Map<String, String> features;
  final List<String> issues;

  bool completed(String run) =>
      predictions[run] == 2 &&
      [
        "queued",
        "running",
        "waiting",
      ].every((phase) => (work[phase] ?? 0) == 0);
}

class ClassificationClient {
  ClassificationClient(this.foreground) {
    lifecycle = LibraryClassificationLifecycle(
      const RustLibraryClassification(),
      foregroundReads: foreground,
    );
    configuration = LibraryClassificationConfiguration(
      const RustLibraryClassificationConfiguration(),
      lifecycle,
    );
    lifecycle.startInBackground();
  }

  final LibraryForegroundReads foreground;
  late final LibraryClassificationLifecycle lifecycle;
  late final LibraryClassificationConfiguration configuration;

  void allowBackground(BigInt revision) => lifecycle.updateCatalog(
    revision: revision,
    availableRoots: const {},
    isBusy: false,
  );

  Future<void> close() async {
    await lifecycle.close().timeout(const Duration(seconds: 15));
    lifecycle.dispose();
  }

  Future<ClassificationObservation> observe() async {
    final result = await Process.run(classificationEnvironment("PYTHON"), [
      "integration_test/support/classification_observer.py",
      Platform.environment["CEDARFLAKE_AME_TEST_STORAGE_ROOT"]!,
    ]).timeout(const Duration(seconds: 5));
    expect(result.exitCode, 0, reason: result.stderr.toString());
    return ClassificationObservation(
      jsonDecode(result.stdout.toString()) as Map<String, dynamic>,
    );
  }

  Future<ClassificationObservation> waitForRun(String run) async {
    final clock = Stopwatch()..start();
    while (clock.elapsed < const Duration(seconds: 120)) {
      expect(lifecycle.error, isNull);
      final state = await observe();
      expect(state.issues, isEmpty);
      if (state.completed(run)) {
        debugPrint("CLASSIFICATION_RUN $run ${clock.elapsedMilliseconds}ms");
        return state;
      }
      await Future<void>.delayed(const Duration(milliseconds: 300));
    }
    throw TestFailure(
      "Classification did not publish both distinct contents for $run",
    );
  }
}
