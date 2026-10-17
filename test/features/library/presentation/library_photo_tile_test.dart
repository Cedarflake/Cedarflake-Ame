import "dart:async";
import "dart:convert";
import "dart:io";

import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_preview_queue.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_query_snapshot_fixture.dart";

void main() {
  for (final sample in [
    (size: const Size(48, 240), textScale: 1.0),
    (size: const Size(240, 48), textScale: 1.0),
    (size: const Size(160, 120), textScale: 3.0),
  ]) {
    testWidgets(
      "compact retry feedback fits ${sample.size} at ${sample.textScale}x text",
      (tester) async {
        final semantics = tester.ensureSemantics();
        try {
          final asset = _failedAsset();
          final snapshot = LibrarySnapshot(
            catalogPath: "C:\\AmeData\\ame.sqlite3",
            revision: BigInt.one,
            queryId: "query-1",
            roots: const [],
            assets: [asset],
          );
          final previewer = _ControlledRetryPreviewer();
          var opens = 0;
          await tester.pumpWidget(
            ProviderScope(
              overrides: [
                initialLibraryStateProvider.overrideWithValue(
                  LibraryState.fromSnapshot(snapshot),
                ),
                libraryCatalogProvider.overrideWithValue(
                  _FakeCatalog(snapshot),
                ),
                libraryScannerProvider.overrideWithValue(const _FakeScanner()),
                libraryPreviewerProvider.overrideWithValue(previewer),
              ],
              child: MaterialApp(
                builder: (context, child) => MediaQuery(
                  data: MediaQuery.of(
                    context,
                  ).copyWith(textScaler: TextScaler.linear(sample.textScale)),
                  child: child!,
                ),
                home: Scaffold(
                  body: LibraryPhotoTile(
                    asset: asset,
                    width: sample.size.width,
                    height: sample.size.height,
                    isSelecting: false,
                    isSelected: false,
                    onOpen: (_) => opens++,
                    onToggleSelection: (_) {},
                    onViewInformation: (_) {},
                    onCopyPath: (_) {},
                    onRevealFile: (_) {},
                  ),
                ),
              ),
            ),
          );
          await tester.pump();
          expect(tester.takeException(), isNull);
          final retry = find.byKey(const Key("preview-retry-location-failed"));
          expect(tester.getSize(retry).shortestSide, greaterThanOrEqualTo(48));
          expect(find.text(LibraryStrings.retryPreview), findsNothing);
          expect(
            find.bySemanticsLabel(RegExp(LibraryStrings.retryPreview)),
            findsWidgets,
          );
          await tester.tap(retry);
          await tester.tap(retry);
          await tester.pump();
          expect(previewer.requests, [asset.locationId]);
          expect(opens, 0);
          expect(find.text(LibraryStrings.retryingPreview), findsNothing);
          expect(
            find.bySemanticsLabel(LibraryStrings.retryingPreview),
            findsOneWidget,
          );
          expect(tester.takeException(), isNull);

          previewer.complete(asset);
          await tester.pump();
          await tester.pump();
          expect(retry, findsOneWidget);
          expect(tester.getSize(find.byType(LibraryPhotoTile)), sample.size);
          expect(tester.takeException(), isNull);
        } finally {
          semantics.dispose();
        }
      },
    );
  }

  test("buckets preview decode widths across small layout changes", () {
    expect(libraryPreviewDecodeWidth(40, 1), 128);
    expect(libraryPreviewDecodeWidth(127, 1), 128);
    expect(libraryPreviewDecodeWidth(129, 1), 256);
    expect(libraryPreviewDecodeWidth(180, 1), 256);
    expect(libraryPreviewDecodeWidth(181, 1), 256);
    expect(libraryPreviewDecodeWidth(129, 2), 512);
    expect(libraryPreviewDecodeWidth(600, 2), 512);
  });

  testWidgets("creates the photo context menu only on demand", (tester) async {
    final asset = _pendingAsset();
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "query-1",
      roots: const [],
      assets: [asset],
    );
    LibraryAsset? copiedAsset;

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(snapshot),
          ),
          libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
          libraryScannerProvider.overrideWithValue(const _FakeScanner()),
          libraryPreviewerProvider.overrideWithValue(
            _RecordingPreviewer(asset),
          ),
        ],
        child: MaterialApp(
          home: Scaffold(
            body: LibraryPhotoTile(
              asset: asset,
              width: 160,
              height: 120,
              isSelecting: false,
              isSelected: false,
              onOpen: (_) {},
              onToggleSelection: (_) {},
              onViewInformation: (_) {},
              onCopyPath: (value) => copiedAsset = value,
              onRevealFile: (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    expect(find.byType(MenuAnchor), findsNothing);

    await tester.tap(
      find.byType(LibraryPhotoTile),
      buttons: kSecondaryMouseButton,
    );
    await tester.pumpAndSettle();

    expect(find.byType(AmeMenuItemContent), findsNWidgets(4));
    expect(find.byType(MenuAnchor), findsNothing);

    await tester.tap(find.text(LibraryStrings.copyPath));
    await tester.pumpAndSettle();

    expect(copiedAsset, same(asset));
    expect(find.byType(AmeMenuItemContent), findsNothing);

    await tester.sendKeyEvent(LogicalKeyboardKey.tab);
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.contextMenu);
    await tester.pumpAndSettle();

    expect(find.byType(AmeMenuItemContent), findsNWidgets(4));
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
  });

  testWidgets("shows one immediate progress state for an explicit retry", (
    tester,
  ) async {
    final asset = _failedAsset();
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "query-1",
      roots: const [],
      assets: [asset],
    );
    final previewer = _ControlledRetryPreviewer();

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(snapshot),
          ),
          libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
          libraryScannerProvider.overrideWithValue(const _FakeScanner()),
          libraryPreviewerProvider.overrideWithValue(previewer),
        ],
        child: MaterialApp(
          home: Scaffold(
            body: LibraryPhotoTile(
              asset: asset,
              width: 160,
              height: 120,
              isSelecting: false,
              isSelected: false,
              onOpen: (_) {},
              onToggleSelection: (_) {},
              onViewInformation: (_) {},
              onCopyPath: (_) {},
              onRevealFile: (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    final retryButton = find.byKey(const Key("preview-retry-location-failed"));
    await tester.tap(retryButton);
    await tester.tap(retryButton);
    await tester.pump();

    expect(previewer.requests, ["location-failed"]);
    expect(
      find.byKey(const Key("preview-retry-progress-location-failed")),
      findsOneWidget,
    );
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(find.text(LibraryStrings.retryingPreview), findsOneWidget);
    expect(retryButton, findsNothing);

    previewer.complete(
      asset.withPreview(
        previewPath: "",
        width: asset.width,
        height: asset.height,
        previewStatus: LibraryPreviewStatus.failed,
        previewIssueCode: "preview_decode_failed",
        previewIssueMessage: "Could not decode preview",
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(find.byType(CircularProgressIndicator), findsNothing);
    expect(find.text(LibraryStrings.retryingPreview), findsNothing);
    expect(retryButton, findsOneWidget);
  });

  testWidgets("explains that an unproven root requires a library update", (
    tester,
  ) async {
    final asset = _failedAsset();
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "query-1",
      roots: const [],
      assets: [asset],
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(snapshot),
          ),
          libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
          libraryScannerProvider.overrideWithValue(const _FakeScanner()),
          libraryPreviewerProvider.overrideWithValue(
            const _UnprovenRootPreviewer(),
          ),
        ],
        child: MaterialApp(
          home: Scaffold(
            body: LibraryPhotoTile(
              asset: asset,
              width: 240,
              height: 160,
              isSelecting: false,
              isSelected: false,
              onOpen: (_) {},
              onToggleSelection: (_) {},
              onViewInformation: (_) {},
              onCopyPath: (_) {},
              onRevealFile: (_) {},
            ),
          ),
        ),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key("preview-retry-location-failed")));
    await tester.pump();
    await tester.pump();

    expect(find.text(LibraryStrings.previewUpdateRequired), findsOneWidget);
    expect(
      find.byKey(const Key("preview-retry-location-failed")),
      findsOneWidget,
    );
  });

  testWidgets(
    "keeps update guidance visible when a ready preview is corrupt on an unproven root",
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-unproven-ready-preview-",
      );
      addTearDown(() => directory.deleteSync(recursive: true));
      final previewFile = File(
        "${directory.path}${Platform.pathSeparator}broken.jpg",
      );
      previewFile.writeAsBytesSync(const [0xFF, 0xD8, 0xFF]);
      final asset = _readyAsset(previewFile.path);
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [],
        assets: [asset],
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState.fromSnapshot(snapshot),
            ),
            libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
            libraryScannerProvider.overrideWithValue(const _FakeScanner()),
            libraryPreviewerProvider.overrideWithValue(
              const _UnprovenRootPreviewer(),
            ),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: LibraryPhotoTile(
                asset: asset,
                width: 240,
                height: 160,
                isSelecting: false,
                isSelected: false,
                onOpen: (_) {},
                onToggleSelection: (_) {},
                onViewInformation: (_) {},
                onCopyPath: (_) {},
                onRevealFile: (_) {},
              ),
            ),
          ),
        ),
      );
      for (
        var attempt = 0;
        attempt < 20 &&
            find.text(LibraryStrings.previewUpdateRequired).evaluate().isEmpty;
        attempt++
      ) {
        await tester.runAsync(() async {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        });
        await tester.pump();
      }

      expect(find.text(LibraryStrings.previewUpdateRequired), findsOneWidget);
      expect(find.byType(Image), findsNothing);
      expect(
        find.bySemanticsLabel(LibraryStrings.previewUpdateRequired),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key("preview-retry-location-ready")),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key("preview-retry-location-ready")));
      await tester.pump();
      await tester.pump();
      expect(find.text(LibraryStrings.previewUpdateRequired), findsOneWidget);
      expect(find.byType(Image), findsNothing);
    },
    timeout: const Timeout(Duration(seconds: 30)),
  );

  for (final replacementKind in [
    "location",
    "source generation",
    "preview artifact",
  ]) {
    testWidgets("retires a queued repair when the $replacementKind changes", (
      tester,
    ) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-preview-callback-",
      );
      addTearDown(() => directory.deleteSync(recursive: true));
      final previewFile = File(
        "${directory.path}${Platform.pathSeparator}ready.png",
      );
      previewFile.writeAsBytesSync(
        base64Decode(
          "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
        ),
      );
      final asset = _readyAsset(previewFile.path);
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [],
        assets: [asset],
      );
      final catalog = _FakeCatalog(snapshot);
      final initialState = LibraryState.fromSnapshot(snapshot);
      final previewer = _ControlledRetryPreviewer();
      Widget buildTile(LibraryAsset displayed) {
        return ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            libraryScannerProvider.overrideWithValue(const _FakeScanner()),
            libraryPreviewerProvider.overrideWithValue(previewer),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: LibraryPhotoTile(
                asset: displayed,
                width: 160,
                height: 120,
                isSelecting: false,
                isSelected: false,
                onOpen: (_) {},
                onToggleSelection: (_) {},
                onViewInformation: (_) {},
                onCopyPath: (_) {},
                onRevealFile: (_) {},
              ),
            ),
          ),
        );
      }

      await tester.pumpWidget(buildTile(asset));
      final image = find.byType(Image);
      tester.widget<Image>(image).errorBuilder!(
        tester.element(image),
        StateError("The previous preview failed to decode"),
        StackTrace.current,
      );
      final LibraryAsset replacement;
      String? replacementPath;
      switch (replacementKind) {
        case "location":
          replacement = _pendingAsset();
        case "source generation":
          replacement = _readyAsset(
            previewFile.path,
            sourceGeneration: BigInt.two,
          );
        case "preview artifact":
          replacementPath =
              "${directory.path}${Platform.pathSeparator}replacement.png";
          previewFile.copySync(replacementPath);
          final controller = ProviderScope.containerOf(
            tester.element(find.byType(LibraryPhotoTile)),
          ).read(libraryControllerProvider.notifier);
          final retry = controller.retryPreview(asset);
          previewer.complete(_readyAsset(replacementPath));
          expect(await retry, LibraryPreviewRequestOutcome.ready);
          replacement = asset;
        default:
          throw StateError("Unknown replacement fixture");
      }
      await tester.pumpWidget(buildTile(replacement));
      await tester.pump();

      expect(previewer.requests, [
        if (replacementKind == "preview artifact") "location-ready",
      ]);
      expect(find.text(LibraryStrings.retryingPreview), findsNothing);
      if (replacementPath != null) {
        final provider = tester.widget<Image>(find.byType(Image)).image;
        final fileImage = (provider as ResizeImage).imageProvider as FileImage;
        expect(fileImage.file.path, replacementPath);
      }
      expect(
        tester.getSize(find.byType(LibraryPhotoTile)),
        const Size(160, 120),
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  testWidgets(
    "displays a same-path repair completed before the next frame",
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-preview-stream-",
      );
      addTearDown(() => directory.deleteSync(recursive: true));
      final previewFile = File(
        "${directory.path}${Platform.pathSeparator}preview.png",
      );
      previewFile.writeAsBytesSync(const [0xFF, 0xD8, 0xFF]);
      final asset = _readyAsset(previewFile.path);
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [],
        assets: [asset],
      );
      late final _ControlledRetryPreviewer previewer;
      previewer = _ControlledRetryPreviewer(
        onMaterialize: () {
          previewFile.writeAsBytesSync(
            base64Decode(
              "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
            ),
          );
          previewer.complete(asset);
        },
      );
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState.fromSnapshot(snapshot),
            ),
            libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
            libraryScannerProvider.overrideWithValue(const _FakeScanner()),
            libraryPreviewerProvider.overrideWithValue(previewer),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: LibraryPhotoTile(
                asset: asset,
                width: 160,
                height: 120,
                isSelecting: false,
                isSelected: false,
                onOpen: (_) {},
                onToggleSelection: (_) {},
                onViewInformation: (_) {},
                onCopyPath: (_) {},
                onRevealFile: (_) {},
              ),
            ),
          ),
        ),
      );
      for (var attempt = 0; attempt < 40; attempt++) {
        await tester.runAsync(() async {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        });
        await tester.pump();
        final pixels = tester.widgetList<RawImage>(find.byType(RawImage));
        if (pixels.any((image) => image.image != null)) {
          break;
        }
      }

      expect(previewer.requests, ["location-ready"]);
      expect(previewer.forceRequests, [true]);
      final image = tester.widget<RawImage>(find.byType(RawImage));
      expect(image.image, isNotNull);
      expect(image.image!.width, greaterThan(0));
      expect(
        find.byKey(const Key("preview-retry-location-ready")),
        findsNothing,
      );
      expect(find.text(LibraryStrings.retryingPreview), findsNothing);
      expect(
        tester.getSize(find.byType(LibraryPhotoTile)),
        const Size(160, 120),
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
    },
    timeout: const Timeout(Duration(seconds: 30)),
  );

  testWidgets(
    "one explicit retry does not rearm automatic repair when the root is unavailable",
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-unavailable-preview-",
      );
      addTearDown(() => directory.deleteSync(recursive: true));
      final previewFile = File(
        "${directory.path}${Platform.pathSeparator}broken.jpg",
      );
      previewFile.writeAsBytesSync(const [0xFF, 0xD8, 0xFF]);
      final asset = _readyAsset(previewFile.path);
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [],
        assets: [asset],
      );
      final previewer = _ControlledRetryPreviewer();
      var opens = 0;
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState.fromSnapshot(snapshot),
            ),
            libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
            libraryScannerProvider.overrideWithValue(const _FakeScanner()),
            libraryPreviewerProvider.overrideWithValue(previewer),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: LibraryPhotoTile(
                asset: asset,
                width: 160,
                height: 120,
                isSelecting: false,
                isSelected: false,
                onOpen: (_) => opens++,
                onToggleSelection: (_) {},
                onViewInformation: (_) {},
                onCopyPath: (_) {},
                onRevealFile: (_) {},
              ),
            ),
          ),
        ),
      );
      for (
        var attempt = 0;
        attempt < 20 && previewer.requests.isEmpty;
        attempt++
      ) {
        await tester.runAsync(() async {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        });
        await tester.pump();
      }
      expect(previewer.requests, [asset.locationId]);
      previewer.fail(
        const LibraryPreviewFailure(
          code: "preview_root_unavailable",
          message: "The source directory is unavailable",
        ),
      );
      final retry = find.byKey(const Key("preview-retry-location-ready"));
      for (
        var expectedRequests = 1;
        expectedRequests <= 3;
        expectedRequests++
      ) {
        // File decoding uses real I/O. Wait for its visible result, while rejecting
        // extra requests on every frame, then observe the settled failure below.
        for (var frame = 0; frame < 80 && retry.evaluate().isEmpty; frame++) {
          await tester.runAsync(() async {
            await Future<void>.delayed(const Duration(milliseconds: 25));
          });
          await tester.pump();
          expect(
            previewer.requests,
            hasLength(expectedRequests),
            reason: "No automatic repair after request $expectedRequests",
          );
        }
        expect(
          retry,
          findsOneWidget,
          reason: "Retry becomes available after request $expectedRequests",
        );
        for (var frame = 0; frame < 5; frame++) {
          await tester.runAsync(() async {
            await Future<void>.delayed(const Duration(milliseconds: 25));
          });
          await tester.pump();
        }
        expect(previewer.requests, hasLength(expectedRequests));
        expect(previewer.forceRequests, everyElement(isTrue));
        expect(retry, findsOneWidget);
        expect(find.text(LibraryStrings.retryingPreview), findsNothing);
        expect(opens, 0);
        expect(tester.takeException(), isNull);
        if (expectedRequests < 3) {
          await tester.tap(retry);
          await tester.pump();
        }
      }
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
    },
    timeout: const Timeout(Duration(seconds: 30)),
  );

  testWidgets(
    "shows automatic repair from the first failed-decode frame",
    (tester) async {
      final directory = Directory.systemTemp.createTempSync(
        "ame-preview-repair-",
      );
      addTearDown(() => directory.deleteSync(recursive: true));
      final previewFile = File(
        "${directory.path}${Platform.pathSeparator}broken.jpg",
      );
      previewFile.writeAsBytesSync(const [0xFF, 0xD8, 0xFF]);
      final asset = _readyAsset(previewFile.path);
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [],
        assets: [asset],
      );
      final previewer = _ControlledRetryPreviewer();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(
              LibraryState.fromSnapshot(snapshot),
            ),
            libraryCatalogProvider.overrideWithValue(_FakeCatalog(snapshot)),
            libraryScannerProvider.overrideWithValue(const _FakeScanner()),
            libraryPreviewerProvider.overrideWithValue(previewer),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: LibraryPhotoTile(
                asset: asset,
                width: 160,
                height: 120,
                isSelecting: false,
                isSelected: false,
                onOpen: (_) {},
                onToggleSelection: (_) {},
                onViewInformation: (_) {},
                onCopyPath: (_) {},
                onRevealFile: (_) {},
              ),
            ),
          ),
        ),
      );
      for (
        var attempt = 0;
        attempt < 20 && previewer.requests.isEmpty;
        attempt++
      ) {
        await tester.runAsync(() async {
          await Future<void>.delayed(const Duration(milliseconds: 25));
        });
        await tester.pump();
        expect(
          find.byKey(const Key("preview-retry-location-ready")),
          findsNothing,
        );
      }
      await tester.pump();

      expect(previewer.requests, ["location-ready"]);
      expect(previewer.forceRequests, [true]);
      expect(
        find.byKey(const Key("preview-retry-progress-location-ready")),
        findsOneWidget,
      );
      previewer.complete(
        asset.withPreview(
          previewPath: asset.previewPath,
          width: asset.width,
          height: asset.height,
          previewStatus: LibraryPreviewStatus.failed,
          previewIssueCode: "preview_repair_failed",
          previewIssueMessage: "Repair failed",
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(find.byKey(const Key("preview-retry-location-ready")), findsOne);
      expect(previewer.requests, ["location-ready"]);
      expect(find.text(LibraryStrings.retryingPreview), findsNothing);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
    },
    timeout: const Timeout(Duration(seconds: 30)),
  );
}

LibraryAsset _readyAsset(String previewPath, {BigInt? sourceGeneration}) {
  return LibraryAsset(
    assetId: "asset-ready",
    locationId: "location-ready",
    rootId: "root-1",
    activeScanId: "scan-1",
    sourceRevision: null,
    sourceGeneration: sourceGeneration ?? BigInt.one,
    sourcePath: "C:\\Pictures\\ready.jpg",
    displayPath: "C:\\Pictures\\ready.jpg",
    relativePath: "ready.jpg",
    previewPath: previewPath,
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 160,
    height: 120,
    previewStatus: LibraryPreviewStatus.ready,
  );
}

LibraryAsset _pendingAsset() {
  return LibraryAsset(
    assetId: "asset-pending",
    locationId: "location-pending",
    rootId: "root-1",
    activeScanId: "scan-1",
    sourceRevision: null,
    sourceGeneration: BigInt.one,
    sourcePath: "C:\\Pictures\\pending.jpg",
    displayPath: "C:\\Pictures\\pending.jpg",
    relativePath: "pending.jpg",
    previewPath: "",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 160,
    height: 120,
    previewStatus: LibraryPreviewStatus.pending,
  );
}

LibraryAsset _failedAsset() {
  return LibraryAsset(
    assetId: "asset-failed",
    locationId: "location-failed",
    rootId: "root-1",
    activeScanId: "scan-1",
    sourceRevision: null,
    sourceGeneration: BigInt.one,
    sourcePath: "C:\\Pictures\\failed.jpg",
    displayPath: "C:\\Pictures\\failed.jpg",
    relativePath: "failed.jpg",
    previewPath: "",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 160,
    height: 120,
    previewStatus: LibraryPreviewStatus.failed,
    previewIssueCode: "preview_decode_failed",
    previewIssueMessage: "Could not decode preview",
  );
}

class _RecordingPreviewer implements LibraryPreviewer {
  _RecordingPreviewer(this.asset);

  final LibraryAsset asset;
  final requests = <({String locationId, bool retry})>[];

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required String expectedRootId,
    required String expectedScanId,
    required LibrarySourceRevisionEvidence? expectedSourceRevision,
    required BigInt expectedSourceGeneration,
    required int previewEdge,
    bool force = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    requests.add((locationId: locationId, retry: force));
    return Future.value(
      asset.withPreview(
        previewPath: asset.previewPath,
        width: asset.width,
        height: asset.height,
        previewStatus: LibraryPreviewStatus.failed,
        previewIssueCode: "preview_repair_failed",
        previewIssueMessage: "Repair failed",
      ),
    );
  }
}

class _ControlledRetryPreviewer implements LibraryPreviewer {
  _ControlledRetryPreviewer({this.onMaterialize});

  final VoidCallback? onMaterialize;
  final requests = <String>[];
  final forceRequests = <bool>[];
  final Completer<LibraryAsset> _completion = Completer<LibraryAsset>();

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required String expectedRootId,
    required String expectedScanId,
    required LibrarySourceRevisionEvidence? expectedSourceRevision,
    required BigInt expectedSourceGeneration,
    required int previewEdge,
    bool force = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    requests.add(locationId);
    forceRequests.add(force);
    onMaterialize?.call();
    return _completion.future;
  }

  void complete(LibraryAsset asset) => _completion.complete(asset);

  void fail(LibraryPreviewFailure failure) =>
      _completion.completeError(failure);
}

class _UnprovenRootPreviewer implements LibraryPreviewer {
  const _UnprovenRootPreviewer();

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required String expectedRootId,
    required String expectedScanId,
    required LibrarySourceRevisionEvidence? expectedSourceRevision,
    required BigInt expectedSourceGeneration,
    required int previewEdge,
    bool force = false,
    Iterable<String> protectedLocationIds = const [],
  }) {
    return Future.error(
      const LibraryPreviewFailure(
        code: "preview_root_identity_unproven",
        message: "root proof is unavailable",
      ),
    );
  }
}

class _FakeCatalog with LibraryQuerySnapshotFixture implements LibraryCatalog {
  const _FakeCatalog(this.snapshot);

  final LibrarySnapshot snapshot;

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async => snapshot;

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async => snapshot;

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    return LibraryTimeline(
      revision: snapshot.revision,
      queryId: snapshot.queryId,
      totalItems: snapshot.assets.length,
      buckets: [
        LibraryTimeBucket(
          itemCount: snapshot.assets.length,
          aspectRatioSum: snapshot.assets.length.toDouble(),
        ),
      ],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) async => false;
}

class _FakeScanner implements LibraryScanner {
  @override
  Future<void> cancelRetainedScan(String scanId) async {
    throw StateError("This fixture has no retained cancellation command");
  }

  const _FakeScanner();

  @override
  bool cancel(String scanId) => false;

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;

  @override
  bool pause(String scanId) => false;

  @override
  bool suspend(String scanId) => false;

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    return const Stream.empty();
  }

  @override
  Stream<LibraryScanUpdate> resume({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    return const Stream.empty();
  }
}
