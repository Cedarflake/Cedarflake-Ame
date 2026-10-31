import "dart:async";
import "dart:convert";
import "dart:io";

import "package:cedarflake_ame/app/window/ame_analysis_environment.dart";
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
    environment = AmeAnalysisEnvironment(
      onSuspendedChanged: lifecycle.setSuspended,
      onMemoryPressure: lifecycle.memoryPressure,
      onError: lifecycle.reportEnvironmentError,
    );
    unawaited(environment.start());
    lifecycle.startInBackground();
  }

  final LibraryForegroundReads foreground;
  late final LibraryClassificationLifecycle lifecycle;
  late final LibraryClassificationConfiguration configuration;
  late final AmeAnalysisEnvironment environment;

  void allowBackground(BigInt revision) => lifecycle.updateCatalog(
    revision: revision,
    availableRoots: const {},
    isBusy: false,
  );

  Future<void> close() async {
    environment.dispose();
    await lifecycle.close().timeout(const Duration(seconds: 15));
    lifecycle.dispose();
  }

  Future<Object?> windows(String action) async {
    final clock = Stopwatch()..start();
    final process = await Process.start(classificationEnvironment("PYTHON"), [
      "integration_test/support/classification_windows.py",
      "$pid",
      action,
    ]);
    final output = process.stdout.transform(utf8.decoder).join();
    final errors = process.stderr.transform(utf8.decoder).join();
    try {
      final exitCode = await process.exitCode.timeout(
        const Duration(seconds: 7),
      );
      expect(exitCode, 0, reason: await errors);
      return jsonDecode(await output);
    } on TimeoutException {
      process.kill();
      await process.exitCode.timeout(const Duration(seconds: 3));
      throw TestFailure(
        "Windows $action observation timed out after ${clock.elapsedMilliseconds}ms: ${await errors}; ${await output}",
      );
    } finally {
      if (action != "workers" || clock.elapsedMilliseconds > 500) {
        debugPrint(
          "CLASSIFICATION_POWER action=$action elapsedMs=${clock.elapsedMilliseconds}",
        );
      }
    }
  }

  Future<void> waitForWorker({required bool resident}) async {
    final clock = Stopwatch()..start();
    final limit = resident
        ? const Duration(seconds: 45)
        : const Duration(seconds: 2);
    while (clock.elapsed < limit) {
      expect(lifecycle.error, isNull);
      final workers = await windows("workers") as List;
      expect(workers.length, lessThanOrEqualTo(1));
      if (workers.isNotEmpty == resident) {
        if (resident && (await observe()).features.isEmpty) {
          await Future<void>.delayed(const Duration(milliseconds: 100));
          continue;
        }
        expect(clock.elapsed, lessThan(limit));
        debugPrint(
          "CLASSIFICATION_RESIDENCY resident=$resident elapsedMs=${clock.elapsedMilliseconds} workers=${jsonEncode(workers)}",
        );
        return;
      }
      await Future<void>.delayed(const Duration(milliseconds: 100));
    }
    throw TestFailure(
      "Classification worker residency did not become $resident",
    );
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
