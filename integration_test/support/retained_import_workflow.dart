import "dart:async";
import "dart:convert";
import "dart:io";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/unified_library_screen.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void registerRetainedImportWorkflowTests(
  LibrarySynchronization Function() readSynchronization,
) {
  testWidgets(
    "retained import waits without locking navigation and cancels durably",
    (tester) async {
      const scanner = RustLibraryScanner();
      const catalog = RustLibraryCatalog();
      final source = await Directory(
        "${Directory.current.path}/build",
      ).createTemp("integration-retained-");
      final sourceBytes = base64Decode(
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
      );
      const fileCount = 1024;
      for (var index = 0; index < fileCount; index++) {
        await File(
          "${source.path}/fixture-$index.png",
        ).writeAsBytes(sourceBytes);
      }
      final scanId =
          "integration-retained-${DateTime.now().microsecondsSinceEpoch}";
      final stopped = Completer<void>();
      var paused = false;
      var pauseAccepted = false;
      Object? failure;
      final subscription = scanner
          .scan(
            scanId: scanId,
            rootPath: source.path,
            itemLimit: null,
            entryLimit: null,
            previewEdge: 128,
          )
          .listen(
            (event) {
              if (event is LibraryScanStarted) {
                pauseAccepted = scanner.pause(scanId);
              } else if (event is LibraryScanPaused) {
                paused = true;
              } else if (event is LibraryScanFailed) {
                failure = "${event.code}: ${event.message}";
              }
            },
            onError: (Object error) => failure = error,
            onDone: stopped.complete,
          );
      addTearDown(() async {
        if (!stopped.isCompleted) {
          scanner.cancel(scanId);
          await stopped.future.timeout(const Duration(seconds: 15));
        }
        await subscription.cancel();
        if (await source.exists()) {
          await source.delete(recursive: true);
        }
      });
      await stopped.future.timeout(const Duration(seconds: 30));
      expect(failure, isNull);
      expect(pauseAccepted, isTrue);
      expect(paused, isTrue);
      final checkpoint = await scanner.loadPausedScan();
      expect(checkpoint?.scanId, scanId);

      Future<ProviderContainer> restoreApplication() async {
        final snapshot = await catalog.load(
          maxItems: libraryCatalogWindow,
          query: const LibraryGalleryQuery(),
        );
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              initialLibraryStateProvider.overrideWithValue(
                LibraryState.fromSnapshot(snapshot),
              ),
              librarySynchronizationProvider.overrideWithValue(
                readSynchronization(),
              ),
            ],
            child: const AmeApp(),
          ),
        );
        return ProviderScope.containerOf(
          tester.element(find.byType(UnifiedLibraryScreen)),
        );
      }

      final container = await restoreApplication();
      await _pumpUntil(tester, () {
        return container.read(libraryControllerProvider).status ==
            LibraryStatus.paused;
      });
      expect(find.byKey(const Key("library-resume-button")), findsOneWidget);
      expect(find.byKey(const Key("library-cancel-button")), findsOneWidget);
      final retainedRoot = container
          .read(libraryControllerProvider)
          .roots
          .singleWhere((root) => root.path == checkpoint!.rootPath);
      expect(retainedRoot.activeScanId, isNull);

      await tester.tap(find.byKey(const Key("library-sidebar-settings")));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("ame-settings-page")), findsOneWidget);
      await tester.tap(find.byKey(const Key("library-sidebar-library")));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("ame-settings-page")), findsNothing);
      expect(container.read(libraryControllerProvider).scanId, scanId);
      expect((await scanner.loadPausedScan())?.scanId, scanId);

      final controller = container.read(libraryControllerProvider.notifier);
      expect(
        await controller.updateQuery(
          LibraryGalleryQuery(rootId: retainedRoot.id),
        ),
        isTrue,
      );
      await tester.pumpAndSettle();
      expect(
        container.read(libraryControllerProvider).status,
        LibraryStatus.paused,
      );
      expect(container.read(libraryControllerProvider).scanId, scanId);
      expect(
        container.read(libraryControllerProvider).stagedAssetCount,
        checkpoint!.acceptedItems,
      );
      expect(
        (await scanner.loadPausedScan())?.acceptedItems,
        checkpoint.acceptedItems,
      );

      await tester.tap(find.byKey(const Key("library-cancel-button")));
      await _pumpUntil(tester, () {
        final state = container.read(libraryControllerProvider);
        return state.scanId == null && !state.hasRetainedScan;
      });
      expect(container.read(libraryControllerProvider).errorMessage, isNull);
      expect(await scanner.loadPausedScan(), isNull);
      expect(await scanner.loadRecoverableScan(), isNull);
      await _expectSourceUnchanged(source, sourceBytes, fileCount);

      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      final reopened = await restoreApplication();
      await tester.pumpAndSettle();
      expect(reopened.read(libraryControllerProvider).scanId, isNull);
      expect(find.byKey(const Key("library-resume-button")), findsNothing);
      expect(await scanner.loadPausedScan(), isNull);
      expect(await scanner.loadRecoverableScan(), isNull);

      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
      expect(await catalog.unregisterRoot(retainedRoot.id), isTrue);
      await _expectSourceUnchanged(source, sourceBytes, fileCount);
    },
  );
}

Future<void> _expectSourceUnchanged(
  Directory source,
  List<int> bytes,
  int fileCount,
) async {
  expect(await source.list(followLinks: false).length, fileCount);
  for (var index = 0; index < fileCount; index++) {
    final file = File("${source.path}/fixture-$index.png");
    expect(
      await FileSystemEntity.type(file.path, followLinks: false),
      FileSystemEntityType.file,
    );
    expect(await file.readAsBytes(), bytes);
  }
}

Future<void> _pumpUntil(WidgetTester tester, bool Function() condition) async {
  final deadline = DateTime.now().add(const Duration(seconds: 30));
  while (!condition()) {
    if (DateTime.now().isAfter(deadline)) {
      throw TestFailure("Retained-import interaction did not settle");
    }
    await tester.pump(const Duration(milliseconds: 25));
  }
  await tester.pump();
}
