import "dart:convert";
import "dart:io";

import "package:cedarflake_ame/features/library/adapters/directory_picker.dart";
import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/unified_library_screen.dart";
import "package:cedarflake_ame/features/storage/application/storage_settings.dart";
import "package:cedarflake_ame/src/rust/frb_generated.dart";
import "package:file_selector/file_selector.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart";
import "package:flutter_test/flutter_test.dart";
import "package:integration_test/integration_test.dart";

const _fixturePng =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=";

class _InitialDirectoryPicker implements DirectoryPicker {
  const _InitialDirectoryPicker(this.path);

  final String path;

  @override
  Future<String?> pickDirectory() {
    return getDirectoryPath(
      initialDirectory: path,
      confirmButtonText: "Import folder",
    );
  }
}

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() {
    final libraryPath = File(
      "${Directory.current.path}${Platform.pathSeparator}build"
      "${Platform.pathSeparator}windows${Platform.pathSeparator}x64"
      "${Platform.pathSeparator}runner${Platform.pathSeparator}Debug"
      "${Platform.pathSeparator}rust_lib_cedarflake_ame.dll",
    ).absolute.path;
    return RustLib.init(
      externalLibrary: ExternalLibrary.open(
        libraryPath,
        debugInfo: "Windows integration Debug library",
      ),
    );
  });

  testWidgets("opens and cancels the production Windows directory picker", (
    tester,
  ) async {
    await tester.pumpWidget(const ProviderScope(child: AmeApp()));

    final pickerAutomation = await _startPickerCancellationAutomation();
    final output = pickerAutomation.stdout
        .transform(const Utf8Decoder(allowMalformed: true))
        .join();
    final error = pickerAutomation.stderr
        .transform(const Utf8Decoder(allowMalformed: true))
        .join();
    addTearDown(() => pickerAutomation.kill());

    await tester.tap(find.byKey(const Key("library-sidebar-import")));
    await tester.pump();

    final libraryContext = tester.element(find.byType(UnifiedLibraryScreen));
    final container = ProviderScope.containerOf(libraryContext);

    final exitCode = await pickerAutomation.exitCode.timeout(
      const Duration(seconds: 15),
      onTimeout: () {
        pickerAutomation.kill();
        return -1;
      },
    );
    final automationOutput = await output;
    final automationError = await error;
    expect(exitCode, 0, reason: "$automationOutput\n$automationError");
    expect(automationOutput, contains("NATIVE_PICKER_CANCELLED"));

    await _pumpUntil(
      tester,
      () =>
          container.read(libraryControllerProvider).status ==
          LibraryStatus.empty,
      timeout: const Duration(seconds: 10),
    );
    expect(find.byKey(const Key("library-empty-state")), findsOneWidget);
    expect(find.text(LibraryStrings.emptyLibraryTitle), findsOneWidget);

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
  });

  testWidgets("selects and scans a controlled folder through the real picker", (
    tester,
  ) async {
    final sourceDirectory = await Directory(
      "${Directory.current.path}${Platform.pathSeparator}build"
      "${Platform.pathSeparator}integration-fixture-"
      "${DateTime.now().microsecondsSinceEpoch}",
    ).create(recursive: true);
    final validSource = File(
      "${sourceDirectory.path}${Platform.pathSeparator}像素.data",
    );
    final corruptSource = File(
      "${sourceDirectory.path}${Platform.pathSeparator}损坏.jpg",
    );
    final secondSourceDirectory = await Directory(
      "${Directory.current.path}${Platform.pathSeparator}build"
      "${Platform.pathSeparator}integration-fixture-second-"
      "${DateTime.now().microsecondsSinceEpoch}",
    ).create(recursive: true);
    final secondValidSource = File(
      "${secondSourceDirectory.path}${Platform.pathSeparator}second.png",
    );
    final validBytes = base64Decode(_fixturePng);
    final corruptBytes = utf8.encode("not an image");

    await validSource.writeAsBytes(validBytes, flush: true);
    await corruptSource.writeAsBytes(corruptBytes, flush: true);
    await secondValidSource.writeAsBytes(validBytes, flush: true);

    addTearDown(() async {
      if (await sourceDirectory.exists()) {
        await sourceDirectory.delete(recursive: true);
      }
      if (await secondSourceDirectory.exists()) {
        await secondSourceDirectory.delete(recursive: true);
      }
    });

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          directoryPickerProvider.overrideWithValue(
            _InitialDirectoryPicker(sourceDirectory.path),
          ),
        ],
        child: const AmeApp(),
      ),
    );

    expect(find.byKey(const Key("library-sidebar-import")), findsOneWidget);
    expect(find.text("Read-only validation"), findsNothing);

    final libraryContext = tester.element(find.byType(UnifiedLibraryScreen));
    final container = ProviderScope.containerOf(libraryContext);

    final pickerAutomation = await _startPickerConfirmationAutomation();
    final output = pickerAutomation.stdout
        .transform(const Utf8Decoder(allowMalformed: true))
        .join();
    final error = pickerAutomation.stderr
        .transform(const Utf8Decoder(allowMalformed: true))
        .join();
    addTearDown(() => pickerAutomation.kill());

    await tester.tap(find.byKey(const Key("library-sidebar-import")));
    await tester.pump();

    final exitCode = await pickerAutomation.exitCode.timeout(
      const Duration(seconds: 15),
      onTimeout: () {
        pickerAutomation.kill();
        return -1;
      },
    );
    final automationOutput = await output;
    final automationError = await error;
    expect(exitCode, 0, reason: "$automationOutput\n$automationError");
    expect(automationOutput, contains("NATIVE_PICKER_CONFIRMED"));

    await _pumpUntil(
      tester,
      () =>
          container.read(libraryControllerProvider).status ==
          LibraryStatus.completed,
      timeout: const Duration(seconds: 30),
    );

    await _pumpUntil(tester, () {
      final current = container.read(libraryControllerProvider);
      return current.assets.length == 1 &&
          container
                  .read(libraryControllerProvider.notifier)
                  .resolvePreview(current.assets.single)
                  .previewStatus ==
              LibraryPreviewStatus.ready;
    }, timeout: const Duration(seconds: 30));
    final state = container.read(libraryControllerProvider);

    expect(state.status, LibraryStatus.completed);
    expect(
      _normalizedPath(state.rootPath!),
      _normalizedPath(sourceDirectory.path),
    );
    expect(state.assets, hasLength(1));
    expect(state.roots, hasLength(1));
    expect(state.roots.single.availability, LibraryRootAvailability.available);
    expect(state.issueCount, 1);
    expect(state.isScanLimited, isFalse);
    expect(find.byKey(const Key("library-photo-wall")), findsOneWidget);

    final asset = container
        .read(libraryControllerProvider.notifier)
        .resolvePreview(state.assets.single);
    final catalogPath = state.catalogPath;
    expect(catalogPath, isNotNull);
    expect(asset.metadataEngineId, "kamadak-exif");
    expect(asset.metadataEngineVersion, "0.6.1+ame-orientation-1");
    expect(asset.captureTime, isNull);
    expect(asset.fileIdentity?.scheme, "windows-file-id-128-v1");
    expect(await File(asset.previewPath).exists(), isTrue);
    expect(await File(catalogPath!).exists(), isTrue);
    expect(_isWithin(sourceDirectory.path, asset.previewPath), isFalse);
    expect(_isWithin(sourceDirectory.path, catalogPath), isFalse);

    expect(await validSource.readAsBytes(), validBytes);
    expect(await corruptSource.readAsBytes(), corruptBytes);
    expect(await sourceDirectory.list().length, 2);

    final storageStatus = await const RustStorageSettingsGateway().load();
    expect(storageStatus.activeCatalogPath, catalogPath);
    expect(storageStatus.settingsPath, isNotEmpty);

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();

    await File(asset.previewPath).delete();
    expect(await File(asset.previewPath).exists(), isFalse);

    const catalog = RustLibraryCatalog();
    const query = LibraryGalleryQuery();
    final restoredSnapshot = await catalog.load(
      maxItems: libraryCatalogWindow,
      query: query,
    );
    final restoredState = LibraryState.fromSnapshot(
      restoredSnapshot,
      query: query,
    );
    expect(restoredState.roots, hasLength(1));
    expect(restoredState.assets, hasLength(1));
    expect(
      restoredState.assets.single.previewStatus,
      LibraryPreviewStatus.pending,
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          directoryPickerProvider.overrideWithValue(
            _InitialDirectoryPicker(secondSourceDirectory.path),
          ),
          libraryCatalogProvider.overrideWithValue(catalog),
          initialLibraryStateProvider.overrideWithValue(restoredState),
        ],
        child: const AmeApp(),
      ),
    );
    await _pumpUntil(tester, () {
      final currentContainer = ProviderScope.containerOf(
        tester.element(find.byType(UnifiedLibraryScreen)),
      );
      final current = currentContainer.read(libraryControllerProvider);
      return currentContainer
              .read(libraryControllerProvider.notifier)
              .resolvePreview(current.assets.single)
              .previewStatus ==
          LibraryPreviewStatus.ready;
    }, timeout: const Duration(seconds: 30));

    expect(find.byKey(const Key("library-photo-wall")), findsOneWidget);

    final secondPickerAutomation = await _startPickerConfirmationAutomation();
    final secondOutput = secondPickerAutomation.stdout
        .transform(const Utf8Decoder(allowMalformed: true))
        .join();
    final secondError = secondPickerAutomation.stderr
        .transform(const Utf8Decoder(allowMalformed: true))
        .join();
    addTearDown(() => secondPickerAutomation.kill());

    await tester.tap(find.byKey(const Key("library-sidebar-import")));
    await tester.pump();

    final secondExitCode = await secondPickerAutomation.exitCode.timeout(
      const Duration(seconds: 15),
      onTimeout: () {
        secondPickerAutomation.kill();
        return -1;
      },
    );
    final secondAutomationOutput = await secondOutput;
    final secondAutomationError = await secondError;
    expect(
      secondExitCode,
      0,
      reason: "$secondAutomationOutput\n$secondAutomationError",
    );
    expect(secondAutomationOutput, contains("NATIVE_PICKER_CONFIRMED"));

    final restoredContext = tester.element(find.byType(UnifiedLibraryScreen));
    final restoredContainer = ProviderScope.containerOf(restoredContext);
    await _pumpUntil(tester, () {
      final current = restoredContainer.read(libraryControllerProvider);
      return current.status == LibraryStatus.completed &&
          current.roots.length == 2;
    }, timeout: const Duration(seconds: 30));

    await _pumpUntil(tester, () {
      final current = restoredContainer.read(libraryControllerProvider);
      final controller = restoredContainer.read(
        libraryControllerProvider.notifier,
      );
      return current.assets.length == 2 &&
          current.assets.every((asset) {
            return controller.resolvePreview(asset).previewStatus ==
                LibraryPreviewStatus.ready;
          });
    }, timeout: const Duration(seconds: 30));
    final previewedMultiRootState = restoredContainer.read(
      libraryControllerProvider,
    );
    expect(previewedMultiRootState.assets, hasLength(2));
    expect(previewedMultiRootState.roots, hasLength(2));
    expect(
      previewedMultiRootState.roots.every(
        (root) => root.availability == LibraryRootAvailability.available,
      ),
      isTrue,
    );
    expect(await secondValidSource.readAsBytes(), validBytes);
    expect(await secondSourceDirectory.list().length, 1);

    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();

    final secondRestoredState = LibraryState.fromSnapshot(
      await catalog.load(maxItems: libraryCatalogWindow, query: query),
      query: query,
    );
    expect(secondRestoredState.roots, hasLength(2));
    expect(secondRestoredState.assets, hasLength(2));
  });
}

Future<void> _pumpUntil(
  WidgetTester tester,
  bool Function() condition, {
  required Duration timeout,
}) async {
  final deadline = DateTime.now().add(timeout);
  while (!condition()) {
    if (DateTime.now().isAfter(deadline)) {
      throw TestFailure("Timed out waiting for the library scan to complete");
    }
    await tester.pump(const Duration(milliseconds: 50));
  }
}

bool _isWithin(String rootPath, String candidatePath) {
  final root = _normalizedPath(rootPath);
  final candidate = _normalizedPath(candidatePath);
  return candidate == root || candidate.startsWith("$root\\");
}

String _normalizedPath(String path) {
  const extendedPathPrefix = "\\\\?\\";
  final normalized = File(path).absolute.path
      .replaceAll("/", "\\")
      .toLowerCase()
      .replaceFirst(RegExp(r"\\+$"), "");
  return normalized.startsWith(extendedPathPrefix)
      ? normalized.substring(extendedPathPrefix.length)
      : normalized;
}

Future<Process> _startPickerCancellationAutomation() {
  final scriptPath =
      "${Directory.current.path}${Platform.pathSeparator}integration_test"
      "${Platform.pathSeparator}support${Platform.pathSeparator}"
      "control_native_directory.ps1";

  return Process.start("powershell.exe", [
    "-NoLogo",
    "-NoProfile",
    "-NonInteractive",
    "-File",
    scriptPath,
    "-TargetProcessId",
    "$pid",
    "-Action",
    "Cancel",
  ]);
}

Future<Process> _startPickerConfirmationAutomation() {
  final scriptPath =
      "${Directory.current.path}${Platform.pathSeparator}integration_test"
      "${Platform.pathSeparator}support${Platform.pathSeparator}"
      "control_native_directory.ps1";

  return Process.start("powershell.exe", [
    "-NoLogo",
    "-NoProfile",
    "-NonInteractive",
    "-File",
    scriptPath,
    "-TargetProcessId",
    "$pid",
  ]);
}
