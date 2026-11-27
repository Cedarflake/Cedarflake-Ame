import "dart:io";

import "package:cedarflake_ame/app/bootstrap/library_synchronization_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/rust_library_loader.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_catalog.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_classification_terms.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_classification_terms.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/application/ports/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_classification_term_editor.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_classification_terms_sheet.dart";
import "package:cedarflake_ame/src/rust/api/classification.dart" as native;
import "package:cedarflake_ame/src/rust/frb_generated.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:integration_test/integration_test.dart";

import "../test/support/localized_test_app.dart";
import "support/classification_client.dart";

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  testWidgets("term forms persist and semantic edits reuse real image features", (
    tester,
  ) async {
    await initializeRustLibrary();
    addTearDown(RustLib.dispose);
    final synchronization =
        LibrarySynchronizationBindings.production().lifecycle;
    addTearDown(synchronization.close);
    synchronization.startInBackground();
    expect(
      await synchronization.startOperation,
      LibrarySynchronizationStartResult.started,
    );
    final foreground = LibraryForegroundReads();
    addTearDown(foreground.dispose);
    final client = ClassificationClient(foreground);
    const service = RustLibraryClassificationTerms();
    final editor = LibraryClassificationTerms(
      service,
      changeConfiguration: client.lifecycle.changeConfiguration,
      onChanged: () {},
    );
    final directory = await service.load();
    expect(directory, hasLength(56));
    expect(directory.every((term) => !term.isConfigured), isTrue);
    expect((await client.configuration.load()).categories, isEmpty);
    final source = Directory(classificationEnvironment("FIXTURE"));
    final originals = <String, List<int>>{};
    addTearDown(() async {
      await client.close();
      for (final entry in originals.entries) {
        expect(await File(entry.key).readAsBytes(), entry.value);
      }
      expect(await source.list(followLinks: false).length, originals.length);
    });
    await for (final entry in source.list(followLinks: false)) {
      expect(entry, isA<File>());
      originals[entry.path] = await (entry as File).readAsBytes();
    }
    expect(originals.length, lessThanOrEqualTo(8));
    expect(
      originals.keys.where(
        (path) => path.endsWith(".jpg") || path.endsWith(".png"),
      ),
      hasLength(3),
    );
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          libraryClassificationTermsProvider.overrideWithValue(editor),
        ],
        child: LocalizedTestApp(
          locale: const Locale("en"),
          theme: buildAmeTheme(locale: const Locale("en")),
          home: Scaffold(
            body: Builder(
              builder: (context) => TextButton(
                onPressed: () => showLibraryClassificationTerms(context),
                child: const Text("Open terms"),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text("Open terms"));
    await _wait(tester, () => find.byType(ListView).evaluate().isNotEmpty);
    await _openTerm(tester, "photo");
    await tester.enterText(
      find.byKey(const Key("classification-term-name")),
      "My photographs",
    );
    await _save(tester);
    var settings = await client.configuration.load();
    expect(settings.installationSelected, isFalse);
    expect(settings.categories, hasLength(6));
    expect(
      settings.categories
          .singleWhere((category) => category.id == "photo")
          .label,
      "My photographs",
    );
    await client.configuration.changeAcceleration(gpuEnabled: false);
    await client.lifecycle.changeConfiguration(
      () => native.selectClassificationInstallation(
        models: classificationEnvironment("MODELS"),
        runtime: classificationEnvironment("RUNTIME"),
      ),
    );
    final events = await const RustLibraryScanner()
        .scan(
          scanId: "classification-term-fixture",
          rootPath: source.path,
          itemLimit: 3,
          entryLimit: originals.length + 1,
          previewEdge: 128,
        )
        .toList()
        .timeout(const Duration(seconds: 30));
    expect(events.whereType<LibraryScanFailed>(), isEmpty);
    expect(events.whereType<LibraryScanCompleted>().single.assetCount, 3);
    final catalog = await const RustLibraryCatalog().load(
      maxItems: 10,
      query: const LibraryGalleryQuery(),
    );
    client.allowBackground(catalog.revision);
    settings = await client.configuration.enable(
      operationId: "terms-initial-run",
      observed: await client.configuration.load(),
    );
    final initial = await client.waitForRun(settings.runId!);

    final dormant = (await service.load()).firstWhere(
      (term) => !term.isConfigured && term.defaultCategory != null,
    );
    final beforeActivation = settings.categories.length;
    await _openTerm(tester, dormant.category.id);
    await _save(tester);
    settings = await client.configuration.load();
    expect(settings.categories, hasLength(beforeActivation + 1));
    expect(
      settings.categories.any((term) => term.id == dormant.category.id),
      isTrue,
    );
    expect(settings.runId, isNot("terms-initial-run"));
    final activated = await client.waitForRun(settings.runId!);
    for (final entry in initial.features.entries) {
      expect(activated.features[entry.key], entry.value);
    }
    debugPrint(
      "CLASSIFICATION_TERMS dormantActivation=true retainedFeaturePayloads=true",
    );

    await _openTerm(tester, "photo");
    await tester.enterText(
      find.byType(TextFormField).last,
      "a close-up photograph of a colorful object",
    );
    await _save(tester);
    settings = await client.configuration.load();
    expect(settings.runId, isNot("terms-initial-run"));
    final rematched = await client.waitForRun(settings.runId!);
    for (final entry in activated.features.entries) {
      expect(rematched.features[entry.key], entry.value);
    }
    expect(rematched.features.length, activated.features.length + 1);
    debugPrint(
      "CLASSIFICATION_TERMS retainedFeaturePayloads=true addedTextFeature=1",
    );

    await _openTerm(tester, "photo");
    await _click(tester, "classification-term-reset");
    await _wait(
      tester,
      () => find.byType(LibraryClassificationTermEditor).evaluate().isEmpty,
    );
    final reset = (await service.load()).singleWhere(
      (term) => term.category.id == "photo",
    );
    expect(reset.category.descriptions, reset.defaultCategory!.descriptions);
    expect(reset.category.label, reset.defaultCategory!.label);

    await _click(tester, "classification-term-add");
    await tester.enterText(
      find.byKey(const Key("classification-term-name")),
      "Sofa cats",
    );
    await tester.enterText(
      find.byType(TextFormField).last,
      "a photo of a cat sleeping on a sofa",
    );
    await _save(tester);
    final custom = (await service.load()).singleWhere((term) => term.isCustom);
    await _openTerm(tester, "photo");
    await tester.enterText(
      find.byType(TextFormField).last,
      "a photograph of a colorful object on a table",
    );
    await _save(tester);
    await _openTerm(tester, dormant.category.id);
    await tester.enterText(
      find.byKey(const Key("classification-term-name")),
      "My activated term",
    );
    await _save(tester);
    final beforeReset = await client.configuration.load();
    final beforeResetFeatures = await client.waitForRun(beforeReset.runId!);
    final configured = beforeReset.categories.map((term) => term.id).toSet();
    await _click(tester, "classification-terms-filter-custom");
    await _click(tester, "classification-terms-more");
    await _click(tester, "classification-terms-reset-all");
    await _click(tester, "classification-terms-reset-confirm");
    await _wait(
      tester,
      () => find
          .byKey(const Key("classification-terms-reset-dialog"))
          .evaluate()
          .isEmpty,
    );
    final afterReset = await client.configuration.load();
    expect(
      afterReset.categories.map((term) => term.id),
      unorderedEquals(configured),
    );
    expect(afterReset.runId, isNot(beforeReset.runId));
    final afterDirectory = await service.load();
    for (final id in ["photo", dormant.category.id]) {
      final term = afterDirectory.singleWhere((term) => term.category.id == id);
      expect(term.category.label, term.defaultCategory!.label);
      expect(term.category.descriptions, term.defaultCategory!.descriptions);
    }
    final retainedCustom = afterDirectory.singleWhere((term) => term.isCustom);
    expect(retainedCustom.category.id, custom.category.id);
    expect(retainedCustom.category.label, custom.category.label);
    expect(retainedCustom.category.descriptions, custom.category.descriptions);
    final resetFeatures = await client.waitForRun(afterReset.runId!);
    for (final entry in beforeResetFeatures.features.entries) {
      expect(resetFeatures.features[entry.key], entry.value);
    }
    debugPrint(
      "CLASSIFICATION_TERMS batchReset=true customPreserved=true enabledIdsPreserved=true retainedFeaturePayloads=true",
    );
    await _openTerm(tester, custom.category.id);
    await _click(tester, "classification-term-delete");
    await _click(tester, "classification-term-delete-confirm");
    await _wait(
      tester,
      () => find.byType(LibraryClassificationTermEditor).evaluate().isEmpty,
    );
    expect((await service.load()).any((term) => term.isCustom), isFalse);
    expect(client.lifecycle.error, isNull);
    debugPrint(
      "CLASSIFICATION_TERMS createEditResetDelete=true sourceFiles=${originals.length}",
    );
  }, timeout: const Timeout(Duration(minutes: 5)));
}

Future<void> _wait(WidgetTester tester, bool Function() ready) async {
  final clock = Stopwatch()..start();
  while (!ready() && clock.elapsed < const Duration(seconds: 20)) {
    await tester.pump(const Duration(milliseconds: 100));
  }
  expect(ready(), isTrue);
  await tester.pumpAndSettle();
}

Future<void> _click(WidgetTester tester, String key) async {
  final target = find.byKey(Key(key));
  await tester.ensureVisible(target);
  await tester.tap(target);
  await tester.pumpAndSettle();
}

Future<void> _save(WidgetTester tester) async {
  await _click(tester, "classification-term-save");
  await _wait(
    tester,
    () => find.byType(LibraryClassificationTermEditor).evaluate().isEmpty,
  );
}

Future<void> _openTerm(WidgetTester tester, String id) async {
  final list = find.byType(ListView);
  tester.widget<ListView>(list).controller!.jumpTo(0);
  await tester.pumpAndSettle();
  await tester.scrollUntilVisible(
    find.byKey(Key("classification-term-$id")),
    180,
    maxScrolls: 40,
    scrollable: find.descendant(of: list, matching: find.byType(Scrollable)),
  );
  await _click(tester, "classification-term-$id");
}
