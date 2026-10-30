import "dart:io";

import "package:cedarflake_ame/app/bootstrap/library_synchronization_lifecycle_owner.dart";
import "package:cedarflake_ame/app/bootstrap/rust_library_loader.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/src/rust/frb_generated.dart";
import "package:flutter/widgets.dart";
import "package:flutter_test/flutter_test.dart";
import "package:integration_test/integration_test.dart";

import "support/classification_client.dart";

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets("real classification preserves control and cached work", (
    tester,
  ) async {
    await initializeRustLibrary();
    addTearDown(() => RustLib.dispose());
    final synchronization = LibrarySynchronizationLifecycleOwner(
      RustLibrarySynchronization.production(),
    );
    addTearDown(synchronization.close);
    synchronization.startInBackground();
    expect(
      await synchronization.startOperation,
      LibrarySynchronizationStartResult.started,
    );
    await tester.pumpWidget(const SizedBox.shrink());
    final source = Directory(classificationEnvironment("FIXTURE"));
    final originals = <String, List<int>>{};
    await for (final entry in source.list(followLinks: false)) {
      expect(entry, isA<File>());
      originals[entry.path] = await (entry as File).readAsBytes();
    }
    expect(originals, hasLength(3));
    addTearDown(() async {
      for (final entry in originals.entries) {
        expect(await File(entry.key).readAsBytes(), entry.value);
      }
      expect(await source.list(followLinks: false).length, 3);
    });

    final foreground = LibraryForegroundReads();
    addTearDown(foreground.dispose);
    var client = ClassificationClient(foreground);
    addTearDown(() => client.close());
    var settings = await client.configuration.load();
    expect(settings.mode, LibraryClassificationMode.disabled);
    final categories = await client.configuration.initialCategories();
    settings = await client.configuration.save(
      operationId: "client-save-disabled",
      observed: settings,
      categories: categories,
      action: LibraryClassificationConfigurationAction.save,
    );
    expect(settings.mode, LibraryClassificationMode.disabled);
    expect(settings.installationSelected, isFalse);

    await client.configuration.selectInstallation(
      models: classificationEnvironment("MODELS"),
      runtime: classificationEnvironment("RUNTIME"),
    );
    final events = await const RustLibraryScanner()
        .scan(
          scanId: "classification-client-fixture",
          rootPath: source.path,
          itemLimit: 3,
          entryLimit: 4,
          previewEdge: 128,
        )
        .toList()
        .timeout(const Duration(seconds: 30));
    final failures = events.whereType<LibraryScanFailed>();
    expect(
      failures,
      isEmpty,
      reason: failures
          .map((event) => "${event.code}: ${event.message}")
          .join("; "),
    );
    expect(events.whereType<LibraryScanCompleted>(), hasLength(1));
    final completion = events.whereType<LibraryScanCompleted>().single;
    expect(completion.assetCount, 3);
    expect(completion.issueCount, 0);
    final library = await const RustLibraryCatalog().load(
      maxItems: 10,
      query: const LibraryGalleryQuery(),
    );
    expect(library.assets, hasLength(3));
    client.allowBackground(library.revision);
    final reading = foreground.acquire();
    addTearDown(reading.close);
    settings = await client.configuration.load();
    settings = await client.configuration.save(
      operationId: "client-first-run",
      observed: settings,
      categories: categories,
      action: LibraryClassificationConfigurationAction.enable,
    );
    expect(settings.mode, LibraryClassificationMode.automatic);
    await Future<void>.delayed(const Duration(seconds: 4));
    expect((await client.observe()).predictions, isEmpty);
    expect((await client.observe()).features, isEmpty);

    settings = await client.configuration.changeControl(
      expectedRevision: settings.dispatchRevision,
      action: LibraryClassificationControl.pause,
    );
    expect(settings.mode, LibraryClassificationMode.paused);
    await client.close();
    client = ClassificationClient(foreground);
    client.allowBackground(library.revision);
    settings = await client.configuration.load();
    expect(settings.mode, LibraryClassificationMode.paused);
    reading.close();
    await client.windows("suspend");
    await client.windows("resume");
    await Future<void>.delayed(const Duration(seconds: 4));
    expect((await client.observe()).predictions, isEmpty);
    expect(
      (await client.configuration.load()).mode,
      LibraryClassificationMode.paused,
    );
    expect(await client.windows("workers"), isEmpty);
    settings = await client.configuration.changeControl(
      expectedRevision: settings.dispatchRevision,
      action: LibraryClassificationControl.resume,
    );
    await client.waitForWorker(resident: true);
    final suspendClock = Stopwatch()..start();
    await client.windows("suspend");
    await client.waitForWorker(resident: false);
    expect(suspendClock.elapsed, lessThan(const Duration(seconds: 2)));
    final suspended = await client.observe();
    expect(suspended.features, isNotEmpty);
    expect(suspended.completed("client-first-run"), isFalse);
    await Future<void>.delayed(const Duration(seconds: 4));
    final stillSuspended = await client.observe();
    expect(stillSuspended.predictions, suspended.predictions);
    expect(stillSuspended.features, suspended.features);
    expect(await client.windows("workers"), isEmpty);
    await client.windows("resume");
    final initial = await client.waitForRun("client-first-run");
    expect(initial.predictions, {"client-first-run": 2});
    expect(initial.features, isNotEmpty);
    for (final entry in suspended.features.entries) {
      expect(initial.features[entry.key], entry.value);
    }

    final renamed = [
      LibraryClassificationCategory(
        id: categories.first.id,
        label: "Renamed category",
        descriptions: categories.first.descriptions,
      ),
      ...categories.skip(1),
    ];
    settings = await client.configuration.load();
    settings = await client.configuration.save(
      operationId: "client-display-rename",
      observed: settings,
      categories: renamed,
      action: LibraryClassificationConfigurationAction.save,
    );
    expect(settings.runId, "client-first-run");
    final preserved = await client.observe();
    expect(preserved.predictions, initial.predictions);
    expect(preserved.features, initial.features);

    final edited = [
      LibraryClassificationCategory(
        id: renamed.first.id,
        label: renamed.first.label,
        descriptions: [
          "A standalone drawing with a colorful background.",
          ...renamed.first.descriptions.skip(1),
        ],
      ),
      ...renamed.skip(1),
    ];
    final pressureClock = Stopwatch()..start();
    client.environment.didHaveMemoryPressure();
    await client.waitForWorker(resident: false);
    settings = await client.configuration.save(
      operationId: "client-edited-run",
      observed: settings,
      categories: edited,
      action: LibraryClassificationConfigurationAction.save,
    );
    expect(settings.runId, "client-edited-run");
    final hold = const Duration(seconds: 55) - pressureClock.elapsed;
    if (hold > Duration.zero) {
      await Future<void>.delayed(hold);
    }
    expect((await client.observe()).predictions, initial.predictions);
    expect(await client.windows("workers"), isEmpty);
    final rematched = await client.waitForRun("client-edited-run");
    expect(
      pressureClock.elapsed,
      greaterThanOrEqualTo(const Duration(seconds: 63)),
    );
    debugPrint(
      "CLASSIFICATION_PRESSURE completedAfterMs=${pressureClock.elapsedMilliseconds}",
    );
    expect(rematched.predictions, {
      "client-first-run": 2,
      "client-edited-run": 2,
    });
    for (final entry in initial.features.entries) {
      expect(rematched.features[entry.key], entry.value);
    }
    expect(rematched.features.length, initial.features.length + 1);
    settings = await client.configuration.load();
    settings = await client.configuration.changeControl(
      expectedRevision: settings.dispatchRevision,
      action: LibraryClassificationControl.stop,
    );
    expect(settings.mode, LibraryClassificationMode.stopped);
    await client.close();
    client = ClassificationClient(foreground);
    client.allowBackground(library.revision);
    expect(
      (await client.configuration.load()).mode,
      LibraryClassificationMode.stopped,
    );
    expect((await client.observe()).predictions, rematched.predictions);
    expect(client.lifecycle.error, isNull);
  }, timeout: const Timeout(Duration(minutes: 5)));
}
