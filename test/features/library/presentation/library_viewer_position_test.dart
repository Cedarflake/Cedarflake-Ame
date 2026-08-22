import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/adapters/directory_picker.dart";
import "package:cedarflake_ame/features/library/adapters/windows_library_platform_actions.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_platform_actions.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("preserves gallery position after closing the image viewer", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(_libraryState()),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pumpAndSettle();

    final photoWall = find.byKey(const Key("library-photo-wall"));
    await tester.drag(photoWall, const Offset(0, -1200));
    await tester.pumpAndSettle();

    final scrollable = find.descendant(
      of: photoWall,
      matching: find.byType(Scrollable),
    );
    final positionBefore = tester.state<ScrollableState>(scrollable).position;
    final offsetBefore = positionBefore.pixels;
    expect(offsetBefore, greaterThan(0));

    final tile = find.byType(LibraryPhotoTile).hitTestable().first;
    final tileRect = tester.getRect(tile);
    await tester.tapAt(tileRect.topLeft + const Offset(16, 16));
    await tester.pump();
    final backButton = find.byKey(const Key("viewer-back-button"));
    expect(backButton, findsOneWidget);
    await tester.tap(backButton);
    await tester.pump();

    final restoredScrollable = find.descendant(
      of: find.byKey(const Key("library-photo-wall")),
      matching: find.byType(Scrollable),
    );
    final positionAfter = tester
        .state<ScrollableState>(restoredScrollable)
        .position;
    expect(identical(positionAfter, positionBefore), isTrue);
    expect(positionAfter.pixels, closeTo(offsetBefore, 0.01));
  });

  testWidgets("reveals the active viewer file after adjacent navigation", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final platformActions = _RecordingPlatformActions();

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            _libraryState(
              assetCount: 2,
              sourceRoot: r"\\?\G:\CloudLibrary\图片",
              displayRoot: r"G:\CloudLibrary\图片",
            ),
          ),
          libraryPlatformActionsProvider.overrideWithValue(platformActions),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pumpAndSettle();

    final openingTile = find.byType(LibraryPhotoTile).hitTestable().first;
    final openingTileRect = tester.getRect(openingTile);
    await tester.tapAt(openingTileRect.topLeft + const Offset(16, 16));
    await tester.pump();
    await _revealCurrentViewerFile(tester);
    expect(platformActions.revealedFiles, [r"\\?\G:\CloudLibrary\图片\0.jpg"]);

    await tester.tap(find.byKey(const Key("viewer-next")));
    await tester.pump();
    await _revealCurrentViewerFile(tester);
    expect(platformActions.revealedFiles, [
      r"\\?\G:\CloudLibrary\图片\0.jpg",
      r"\\?\G:\CloudLibrary\图片\1.jpg",
    ]);
  });

  testWidgets(
    "refreshes a catalog revision already published before screen subscription",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _libraryState(assetCount: 1);
      final initialAsset = initialState.assets.single;
      final renamedAsset = _assetAtPath(
        initialAsset,
        locationId: "location-renamed-before-subscription",
        relativePath: "renamed-before-subscription.jpg",
      );
      final catalog = _SynchronizationViewerCatalog(
        LibrarySnapshot(
          catalogPath: initialState.catalogPath ?? "",
          revision: BigInt.two,
          queryId: initialState.queryId,
          roots: initialState.roots,
          assets: [renamedAsset],
        ),
      );
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.two),
      );
      addTearDown(synchronization.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await _pumpSynchronizationRefresh(tester);

      final container = ProviderScope.containerOf(
        tester.element(find.byType(AmeApp)),
      );
      final refreshedState = container.read(libraryControllerProvider);
      expect(refreshedState.catalogRevision, BigInt.two);
      expect(
        refreshedState.assets.single.locationId,
        "location-renamed-before-subscription",
      );
    },
  );

  testWidgets(
    "synchronization refresh failure stops automatic retries and stays visible",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _libraryState(assetCount: 1);
      final catalog = _FailingSynchronizationCatalog();
      final scanner = _HeldLibraryScanner();
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.two),
      );
      addTearDown(synchronization.dispose);
      addTearDown(scanner.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            libraryScannerProvider.overrideWithValue(scanner),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));

      expect(catalog.loadCount, 1);
      expect(
        find.text(LibraryStrings.synchronizationRefreshFailureTitle),
        findsOneWidget,
      );

      await tester.pump(const Duration(seconds: 3));
      expect(catalog.loadCount, 1);

      await tester.tap(find.byKey(const Key("notification-primary-action")));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));
      expect(catalog.loadCount, 2);
      expect(
        find.text(LibraryStrings.synchronizationRefreshFailureTitle),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    "newer synchronization revisions coalesce after an in-flight failure",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _libraryState(assetCount: 1);
      final catalog = _HeldFailureSynchronizationCatalog(
        _snapshotFromState(initialState, revision: BigInt.from(4)),
      );
      final scanner = _HeldLibraryScanner();
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.two),
      );
      addTearDown(synchronization.dispose);
      addTearDown(scanner.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            libraryScannerProvider.overrideWithValue(scanner),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(catalog.loadCount, 1);

      synchronization.publish(_synchronizationSnapshot(BigInt.from(3)));
      synchronization.publish(_synchronizationSnapshot(BigInt.from(4)));
      await tester.pump();
      catalog.failHeldLoad();
      await _pumpSynchronizationRefresh(tester);

      final container = ProviderScope.containerOf(
        tester.element(find.byType(AmeApp)),
      );
      expect(catalog.loadCount, 2);
      expect(
        container.read(libraryControllerProvider).catalogRevision,
        BigInt.from(4),
      );
      expect(
        find.text(LibraryStrings.synchronizationRefreshFailureTitle),
        findsNothing,
      );

      await tester.pump(const Duration(seconds: 3));
      expect(catalog.loadCount, 2);
    },
  );

  testWidgets(
    "automatic retry keeps one error until synchronization proves recovery",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _libraryState(assetCount: 1);
      final scanner = _HeldLibraryScanner();
      final synchronization = _TestLibrarySynchronization(
        _needsReconciliationSnapshot(),
      );
      addTearDown(synchronization.dispose);
      addTearDown(scanner.dispose);
      final catalog = _SynchronizationViewerCatalog(
        _snapshotFromState(initialState, revision: BigInt.two),
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            libraryScannerProvider.overrideWithValue(scanner),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();

      expect(find.text("“Pictures”更新受阻"), findsOneWidget);
      expect(
        find.text("Windows 拒绝了目录监控访问。Ame 会保留上次可信内容，并在恢复后重新核对。"),
        findsOneWidget,
      );
      expect(find.textContaining("阶段：等待故障恢复"), findsOneWidget);
      expect(find.textContaining("3 项等待处理"), findsOneWidget);
      expect(find.textContaining("2 项等待重试"), findsOneWidget);
      expect(find.textContaining("1 项状态尚未确认"), findsOneWidget);
      expect(find.byKey(const Key("notification-unread-icon")), findsOneWidget);

      synchronization.publish(_automaticRecoverySnapshot());
      await tester.pump();

      expect(find.text("“Pictures”更新受阻"), findsOneWidget);
      expect(find.text("短时间内的文件变化超出监控可确认范围，Ame 正在重新核对该目录。"), findsNothing);
      expect(
        find.byKey(const Key("notification-primary-action")),
        findsNothing,
      );

      synchronization.publish(_persistenceFailureSnapshot());
      await tester.pump();
      final container = ProviderScope.containerOf(
        tester.element(find.byType(AmeApp)),
      );
      expect(
        container.read(ameNotificationControllerProvider).history,
        hasLength(1),
      );
      expect(
        container
            .read(ameNotificationControllerProvider)
            .history
            .single
            .technicalCode,
        "catalog_database_busy",
      );

      synchronization.publish(_synchronizedRootSnapshot());
      await tester.pump();

      expect(find.text("“Pictures”更新受阻"), findsNothing);
      expect(
        find.byKey(const Key("notification-primary-action")),
        findsNothing,
      );

      await tester.tap(find.byKey(const Key("notification-history-button")));
      await tester.pumpAndSettle();

      expect(
        find.byKey(const Key("notification-history-item-ame-notification-2")),
        findsNothing,
      );
      await tester.tap(
        find.byKey(const Key("notification-history-item-ame-notification-1")),
      );
      await tester.pumpAndSettle();

      expect(find.text("catalog_database_busy"), findsOneWidget);
      expect(find.text(r"C:\Pictures"), findsWidgets);

      await tester.tap(find.text("关闭"));
      await tester.pumpAndSettle();
      expect(scanner.scanCount, 0);
    },
  );

  testWidgets(
    "catalog persistence failure is reported without starting a manual scan",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _libraryState(assetCount: 1);
      final scanner = _HeldLibraryScanner();
      final synchronization = _TestLibrarySynchronization(
        _persistenceFailureSnapshot(),
      );
      addTearDown(synchronization.dispose);
      addTearDown(scanner.dispose);
      final catalog = _SynchronizationViewerCatalog(
        _snapshotFromState(initialState, revision: BigInt.two),
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            libraryScannerProvider.overrideWithValue(scanner),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();

      expect(find.text("“Pictures”更新受阻"), findsOneWidget);
      expect(
        find.text(LibraryStrings.synchronizationPersistenceFailed),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key("notification-primary-action")),
        findsNothing,
      );
      expect(scanner.scanCount, 0);

      await tester.tap(find.byKey(const Key("notification-history-button")));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const Key("notification-history-item-ame-notification-1")),
      );
      await tester.pumpAndSettle();

      expect(find.text("catalog_database_busy"), findsOneWidget);
      expect(find.text(r"C:\Pictures"), findsWidgets);
      expect(scanner.scanCount, 0);
    },
  );

  testWidgets("normal synchronization and convergence remain silent", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final initialState = _libraryState(assetCount: 1);
    final scanner = _HeldLibraryScanner();
    final synchronization = _TestLibrarySynchronization(
      _automaticRecoverySnapshot(),
    );
    addTearDown(synchronization.dispose);
    addTearDown(scanner.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          libraryCatalogProvider.overrideWithValue(
            _SynchronizationViewerCatalog(_snapshotFromState(initialState)),
          ),
          libraryScannerProvider.overrideWithValue(scanner),
          librarySynchronizationProvider.overrideWithValue(synchronization),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(AmeApp)),
    );
    expect(container.read(ameNotificationControllerProvider).history, isEmpty);

    synchronization.publish(_synchronizedRootSnapshot());
    await tester.pump();

    expect(container.read(ameNotificationControllerProvider).history, isEmpty);
    expect(scanner.scanCount, 0);
  });

  testWidgets("root generation change resolves the prior blocked condition", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final initialState = _libraryState(assetCount: 1);
    final synchronization = _TestLibrarySynchronization(
      _needsReconciliationSnapshot(),
    );
    addTearDown(synchronization.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          libraryCatalogProvider.overrideWithValue(
            _SynchronizationViewerCatalog(_snapshotFromState(initialState)),
          ),
          librarySynchronizationProvider.overrideWithValue(synchronization),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(AmeApp)),
    );
    expect(
      container.read(ameNotificationControllerProvider).history.single.isActive,
      isTrue,
    );

    synchronization.publish(_automaticRecoverySnapshot(generation: BigInt.two));
    await tester.pump();

    final notifications = container.read(ameNotificationControllerProvider);
    expect(notifications.history, hasLength(1));
    expect(notifications.history.single.isActive, isFalse);
    expect(notifications.pendingIds, isEmpty);
  });

  testWidgets(
    "scan feedback takes priority over a pending synchronization failure",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _libraryState(assetCount: 1);
      final catalog = _FailOnceSynchronizationCatalog(
        _snapshotFromState(initialState),
      );
      final scanner = _HeldLibraryScanner();
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.two),
      );
      addTearDown(synchronization.dispose);
      addTearDown(scanner.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            libraryScannerProvider.overrideWithValue(scanner),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      expect(
        find.text(LibraryStrings.synchronizationRefreshFailureTitle),
        findsOneWidget,
      );

      final container = ProviderScope.containerOf(
        tester.element(find.byType(AmeApp)),
      );
      await container
          .read(libraryControllerProvider.notifier)
          .scanDirectory(r"C:\Pictures");
      final scanId = scanner.scanId;
      expect(scanId, isNotNull);
      scanner.add(
        LibraryScanStarted(
          scanId: scanId!,
          rootPath: r"C:\Pictures",
          itemLimit: null,
          entryLimit: null,
        ),
      );
      scanner.add(
        const LibraryScanProgress(
          visitedEntries: 128,
          acceptedItems: 64,
          issueCount: 0,
        ),
      );
      await tester.pump();

      expect(find.text("正在添加文件夹“Pictures”…"), findsOneWidget);
      expect(find.text("已检查 128 个文件 · 已找到 64 张图片"), findsOneWidget);
      expect(find.byKey(const Key("library-pause-button")), findsOneWidget);
      expect(find.byKey(const Key("library-cancel-button")), findsOneWidget);
      expect(find.byKey(const Key("library-retry-button")), findsNothing);
      expect(
        find.text(LibraryStrings.synchronizationRefreshFailureTitle),
        findsNothing,
      );

      scanner.add(
        LibraryScanCompleted(
          assetCount: 64,
          issueCount: 0,
          catalogPath: initialState.catalogPath ?? "",
          wasLimited: false,
        ),
      );
      await scanner.close();
      await tester.pump();
      await tester.pump();

      expect(find.text("导入完成"), findsOneWidget);
      expect(
        find.byKey(const Key("library-task-dismiss-button")),
        findsOneWidget,
      );
      expect(find.byKey(const Key("library-retry-button")), findsNothing);

      await tester.tap(find.byKey(const Key("library-task-dismiss-button")));
      await tester.pump();

      expect(
        find.text(LibraryStrings.synchronizationRefreshFailureTitle),
        findsOneWidget,
      );
      expect(
        find.byKey(const Key("notification-primary-action")),
        findsOneWidget,
      );
    },
  );

  testWidgets(
    "failed scan feedback can be acknowledged before synchronization retry",
    (tester) => _verifyTerminalScanFeedbackAcknowledgement(
      tester,
      terminalUpdate: const LibraryScanFailed(
        code: "catalog_database_busy",
        message: "The catalog database remained busy after waiting",
      ),
      terminalTitle: "添加文件夹失败",
    ),
  );

  testWidgets(
    "cancelled scan feedback can be acknowledged before synchronization retry",
    (tester) => _verifyTerminalScanFeedbackAcknowledgement(
      tester,
      terminalUpdate: const LibraryScanCancelled(
        acceptedItems: 0,
        issueCount: 0,
      ),
      terminalTitle: "已取消添加文件夹",
    ),
  );

  testWidgets(
    "pre-scan failure can be acknowledged before synchronization retry",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _libraryState(assetCount: 1);
      final catalog = _FailingSynchronizationCatalog();
      final scanner = _HeldLibraryScanner();
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.two),
      );
      addTearDown(synchronization.dispose);
      addTearDown(scanner.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            directoryPickerProvider.overrideWithValue(
              const _FailingDirectoryPicker(),
            ),
            libraryCatalogProvider.overrideWithValue(catalog),
            libraryScannerProvider.overrideWithValue(scanner),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(seconds: 1));
      expect(
        find.text(LibraryStrings.synchronizationRefreshFailureTitle),
        findsOneWidget,
      );

      final container = ProviderScope.containerOf(
        tester.element(find.byType(AmeApp)),
      );
      final originalState = container.read(libraryControllerProvider);
      await container
          .read(libraryControllerProvider.notifier)
          .chooseDirectoryAndScan();
      await tester.pump();

      final failedState = container.read(libraryControllerProvider);
      expect(failedState.status, LibraryStatus.failed);
      expect(failedState.scanId, isNull);
      expect(scanner.scanCount, 0);
      expect(find.text("添加文件夹失败"), findsOneWidget);
      expect(find.byKey(const Key("library-retry-button")), findsOneWidget);
      expect(
        find.byKey(const Key("library-task-dismiss-button")),
        findsOneWidget,
      );
      expect(
        find.text(LibraryStrings.synchronizationRefreshFailureTitle),
        findsNothing,
      );

      await tester.tap(find.byKey(const Key("library-task-dismiss-button")));
      await tester.pump();

      final acknowledgedState = container.read(libraryControllerProvider);
      expect(acknowledgedState.status, LibraryStatus.completed);
      expect(acknowledgedState.roots, originalState.roots);
      expect(acknowledgedState.assets, originalState.assets);
      expect(acknowledgedState.catalogRevision, originalState.catalogRevision);
      expect(acknowledgedState.errorMessage, isNull);
      expect(find.text("添加文件夹失败"), findsNothing);
      expect(
        find.text(LibraryStrings.synchronizationRefreshFailureTitle),
        findsOneWidget,
      );

      await tester.tap(find.byKey(const Key("notification-primary-action")));
      await _pumpSynchronizationRefresh(tester);

      expect(catalog.loadCount, 2);
      expect(scanner.scanCount, 0);
    },
  );

  testWidgets(
    "keeps the preferred viewer location until authoritative lookup resolves it",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final baseState = _libraryState(assetCount: 1);
      final firstLocation = baseState.assets.single;
      final preferredLocation = _assetAtPath(
        firstLocation,
        locationId: "location-preferred",
        relativePath: "preferred.jpg",
      );
      final initialState = _stateWithAssets(baseState, [
        firstLocation,
        preferredLocation,
      ]);
      final initialSnapshot = LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: BigInt.one,
        queryId: initialState.queryId,
        roots: initialState.roots,
        assets: initialState.assets,
      );
      final catalog = _SynchronizationViewerCatalog(initialSnapshot);
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.one),
      );
      addTearDown(synchronization.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pumpAndSettle();

      final preferredTile = find.byWidgetPredicate(
        (widget) =>
            widget is LibraryPhotoTile &&
            widget.asset.locationId == preferredLocation.locationId,
      );
      final preferredTileRect = tester.getRect(preferredTile);
      await tester.tapAt(preferredTileRect.topLeft + const Offset(16, 16));
      await tester.pump();
      expect(find.text("preferred.jpg"), findsOneWidget);

      catalog.snapshot = LibrarySnapshot(
        catalogPath: initialSnapshot.catalogPath,
        revision: BigInt.two,
        queryId: initialSnapshot.queryId,
        roots: initialSnapshot.roots,
        assets: [firstLocation],
        queryAnchorResolution: const LibraryQueryAnchorResolution(
          requestedLocationId: "location-preferred",
          locationId: "location-0",
          ordinal: 0,
          windowStartItemOffset: 0,
        ),
      );
      synchronization.publish(_synchronizationSnapshot(BigInt.two));
      await _pumpSynchronizationRefresh(tester);

      expect(catalog.requestedLocationIds, ["location-preferred"]);
      expect(catalog.preferredLocationIds, ["location-preferred"]);
      expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);
      expect(find.text("0.jpg"), findsOneWidget);
    },
  );

  testWidgets(
    "stale asset lookup cannot replace newer same-asset viewer navigation",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final baseState = _libraryState(assetCount: 1);
      final firstLocation = baseState.assets.single;
      final secondLocation = _assetAtPath(
        firstLocation,
        locationId: "location-second",
        relativePath: "second.jpg",
      );
      final initialState = _stateWithAssets(baseState, [
        firstLocation,
        secondLocation,
      ]);
      final initialSnapshot = LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: BigInt.one,
        queryId: initialState.queryId,
        roots: initialState.roots,
        assets: initialState.assets,
      );
      final catalog = _SynchronizationViewerCatalog(initialSnapshot);
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.one),
      );
      addTearDown(synchronization.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pumpAndSettle();

      final firstTile = find.byWidgetPredicate(
        (widget) =>
            widget is LibraryPhotoTile &&
            widget.asset.locationId == firstLocation.locationId,
      );
      final firstTileRect = tester.getRect(firstTile);
      await tester.tapAt(firstTileRect.topLeft + const Offset(16, 16));
      await tester.pump();
      expect(find.text("0.jpg"), findsOneWidget);

      catalog.snapshot = LibrarySnapshot(
        catalogPath: initialSnapshot.catalogPath,
        revision: BigInt.two,
        queryId: initialSnapshot.queryId,
        roots: initialSnapshot.roots,
        assets: [firstLocation, secondLocation],
        queryAnchorResolution: const LibraryQueryAnchorResolution(
          requestedLocationId: "location-0",
          locationId: "location-0",
          ordinal: 0,
          windowStartItemOffset: 0,
        ),
      );
      catalog.holdNextAssetLookup();
      synchronization.publish(_synchronizationSnapshot(BigInt.two));
      await tester.pump(const Duration(milliseconds: 100));
      expect(catalog.preferredLocationIds, ["location-0"]);

      await tester.tap(find.byKey(const Key("viewer-next")));
      await tester.pump();
      expect(find.text("second.jpg"), findsOneWidget);

      catalog.completeHeldAssetLookup(firstLocation);
      await _pumpSynchronizationRefresh(tester);

      expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);
      expect(find.text("second.jpg"), findsOneWidget);
      expect(find.text("0.jpg"), findsNothing);
    },
  );

  testWidgets(
    "synchronization keeps a renamed viewer asset and closes an authoritative removal",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = _libraryState(assetCount: 1);
      final initialAsset = initialState.assets.single;
      final initialSnapshot = LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: initialState.catalogRevision ?? BigInt.one,
        queryId: initialState.queryId,
        roots: initialState.roots,
        assets: initialState.assets,
      );
      final catalog = _SynchronizationViewerCatalog(initialSnapshot);
      final synchronization = _TestLibrarySynchronization(
        _synchronizationSnapshot(BigInt.one),
      );
      addTearDown(synchronization.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            librarySynchronizationProvider.overrideWithValue(synchronization),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pumpAndSettle();

      final openingTile = find.byType(LibraryPhotoTile).hitTestable().first;
      final openingTileRect = tester.getRect(openingTile);
      await tester.tapAt(openingTileRect.topLeft + const Offset(16, 16));
      await tester.pump();
      expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);
      expect(find.text("0.jpg"), findsOneWidget);

      final renamedAsset = _assetAtPath(
        initialAsset,
        locationId: "location-renamed",
        relativePath: "renamed.jpg",
      );
      catalog.snapshot = LibrarySnapshot(
        catalogPath: initialSnapshot.catalogPath,
        revision: BigInt.two,
        queryId: initialSnapshot.queryId,
        roots: initialSnapshot.roots,
        assets: [renamedAsset],
        queryAnchorResolution: const LibraryQueryAnchorResolution(
          requestedLocationId: "location-0",
          locationId: "location-renamed",
          ordinal: 0,
          windowStartItemOffset: 0,
        ),
      );
      synchronization.publish(_synchronizationSnapshot(BigInt.two));
      await _pumpSynchronizationRefresh(tester);

      expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);
      expect(find.text("renamed.jpg"), findsOneWidget);
      expect(catalog.requestedLocationIds, ["location-0"]);
      expect(catalog.preferredLocationIds, ["location-0"]);

      final fallbackAsset = _assetAtPath(
        initialAsset,
        assetId: "asset-fallback",
        locationId: "location-fallback",
        relativePath: "fallback.jpg",
      );
      catalog.snapshot = LibrarySnapshot(
        catalogPath: initialSnapshot.catalogPath,
        revision: BigInt.from(3),
        queryId: initialSnapshot.queryId,
        roots: initialSnapshot.roots,
        assets: [fallbackAsset],
        queryAnchorResolution: const LibraryQueryAnchorResolution(
          requestedLocationId: "location-renamed",
          locationId: "location-fallback",
          ordinal: 0,
          windowStartItemOffset: 0,
        ),
      );
      synchronization.publish(_synchronizationSnapshot(BigInt.from(3)));
      await _pumpSynchronizationRefresh(tester);

      expect(find.byKey(const Key("viewer-back-button")), findsNothing);
      expect(find.byKey(const Key("library-photo-wall")), findsOneWidget);
      expect(catalog.requestedLocationIds, ["location-0", "location-renamed"]);
      expect(catalog.preferredLocationIds, ["location-0", "location-renamed"]);
    },
  );
}

Future<void> _revealCurrentViewerFile(WidgetTester tester) async {
  await tester.tap(find.byKey(const Key("viewer-more-menu")));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 500));
  await tester.tap(find.text(LibraryStrings.openInExplorer));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 500));
}

Future<void> _pumpSynchronizationRefresh(WidgetTester tester) async {
  for (var index = 0; index < 8; index++) {
    await tester.pump(const Duration(milliseconds: 100));
  }
}

Future<void> _verifyTerminalScanFeedbackAcknowledgement(
  WidgetTester tester, {
  required LibraryScanUpdate terminalUpdate,
  required String terminalTitle,
}) async {
  tester.view.physicalSize = const Size(1280, 800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final initialState = _libraryState(assetCount: 1);
  final catalog = _FailingSynchronizationCatalog();
  final scanner = _HeldLibraryScanner();
  final synchronization = _TestLibrarySynchronization(
    _synchronizationSnapshot(BigInt.two),
  );
  addTearDown(synchronization.dispose);
  addTearDown(scanner.dispose);

  await tester.pumpWidget(
    ProviderScope(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(initialState),
        libraryCatalogProvider.overrideWithValue(catalog),
        libraryScannerProvider.overrideWithValue(scanner),
        librarySynchronizationProvider.overrideWithValue(synchronization),
      ],
      child: const AmeApp(),
    ),
  );
  await tester.pump();
  await tester.pump(const Duration(seconds: 1));
  expect(
    find.text(LibraryStrings.synchronizationRefreshFailureTitle),
    findsOneWidget,
  );

  final container = ProviderScope.containerOf(
    tester.element(find.byType(AmeApp)),
  );
  await container
      .read(libraryControllerProvider.notifier)
      .scanDirectory(r"C:\Pictures");
  final scanId = scanner.scanId;
  expect(scanId, isNotNull);
  scanner.add(LibraryScanStarted(scanId: scanId!, rootPath: r"C:\Pictures"));
  scanner.add(terminalUpdate);
  await tester.pump();

  expect(find.text(terminalTitle), findsOneWidget);
  expect(find.byKey(const Key("library-retry-button")), findsOneWidget);
  expect(find.byKey(const Key("library-task-dismiss-button")), findsOneWidget);
  expect(
    find.text(LibraryStrings.synchronizationRefreshFailureTitle),
    findsNothing,
  );
  expect(scanner.scanCount, 1);

  await tester.tap(find.byKey(const Key("library-task-dismiss-button")));
  await tester.pump();

  expect(find.text(terminalTitle), findsNothing);
  expect(
    find.text(LibraryStrings.synchronizationRefreshFailureTitle),
    findsOneWidget,
  );
  expect(find.byKey(const Key("notification-primary-action")), findsOneWidget);

  await tester.tap(find.byKey(const Key("notification-primary-action")));
  await _pumpSynchronizationRefresh(tester);

  expect(catalog.loadCount, 2);
  expect(scanner.scanCount, 1);
  expect(
    find.text(LibraryStrings.synchronizationRefreshFailureTitle),
    findsOneWidget,
  );
}

LibraryState _libraryState({
  int assetCount = 120,
  String sourceRoot = r"C:\Pictures",
  String? displayRoot,
}) {
  final readableRoot = displayRoot ?? sourceRoot;
  final assets = [
    for (var index = 0; index < assetCount; index++)
      LibraryAsset(
        assetId: "asset-$index",
        locationId: "location-$index",
        rootId: "root-1",
        sourcePath: "$sourceRoot\\$index.jpg",
        displayPath: "$readableRoot\\$index.jpg",
        relativePath: "$index.jpg",
        previewPath: "C:\\Ame\\previews\\$index.jpg",
        fileSize: BigInt.one,
        modifiedUnixMs: index,
        width: 4,
        height: 3,
        previewStatus: LibraryPreviewStatus.ready,
      ),
  ];
  return LibraryState.fromSnapshot(
    LibrarySnapshot(
      catalogPath: "C:\\Ame\\catalog.db",
      revision: BigInt.one,
      queryId: "viewer-position",
      roots: [
        LibraryRoot(
          id: "root-1",
          path: sourceRoot,
          displayPath: readableRoot,
          createdUnixMs: 1,
          assetCount: assets.length,
          issueCount: 0,
        ),
      ],
      assets: assets,
    ),
  );
}

LibraryState _stateWithAssets(LibraryState source, List<LibraryAsset> assets) {
  return LibraryState.fromSnapshot(
    LibrarySnapshot(
      catalogPath: source.catalogPath ?? "",
      revision: source.catalogRevision ?? BigInt.one,
      queryId: source.queryId,
      roots: source.roots,
      assets: assets,
    ),
  );
}

LibraryAsset _assetAtPath(
  LibraryAsset source, {
  String? assetId,
  required String locationId,
  required String relativePath,
}) {
  return LibraryAsset(
    assetId: assetId ?? source.assetId,
    locationId: locationId,
    rootId: source.rootId,
    sourcePath: "C:\\Pictures\\$relativePath",
    displayPath: "C:\\Pictures\\$relativePath",
    relativePath: relativePath,
    previewPath: source.previewPath,
    fileSize: source.fileSize,
    createdUnixMs: source.createdUnixMs,
    modifiedUnixMs: source.modifiedUnixMs,
    width: source.width,
    height: source.height,
    previewStatus: source.previewStatus,
    previewIssueCode: source.previewIssueCode,
    previewIssueMessage: source.previewIssueMessage,
    metadataEngineId: source.metadataEngineId,
    metadataEngineVersion: source.metadataEngineVersion,
    captureTime: source.captureTime,
    fileIdentity: source.fileIdentity,
  );
}

LibrarySynchronizationSnapshot _synchronizationSnapshot(BigInt revision) {
  return LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: revision,
    appliedMutationCount: 1,
    roots: const {},
  );
}

LibrarySynchronizationSnapshot _needsReconciliationSnapshot() {
  return LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: BigInt.one,
    appliedMutationCount: 0,
    roots: {
      "root-1": LibraryRootSynchronizationStatus(
        rootId: "root-1",
        rootGeneration: BigInt.one,
        availability: LibraryRootAvailability.available,
        freshness: LibraryCatalogFreshness.needsReconciliation,
        freshnessCause: LibraryCatalogFreshnessCause.changeSourceUnhealthy,
        phase: LibrarySynchronizationPhase.blocked,
        phaseStartedAt: DateTime.utc(2026, 8, 21),
        sourceStatus: LibraryChangeSourceStatus.failed,
        pendingChangeCount: BigInt.from(3),
        retryWaitCount: BigInt.two,
        freshnessUnknownCount: BigInt.one,
        lastIssueCode: "change_source_callback_access_denied",
      ),
    },
  );
}

LibrarySynchronizationSnapshot _automaticRecoverySnapshot({
  BigInt? generation,
}) {
  return LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: BigInt.one,
    appliedMutationCount: 0,
    roots: {
      "root-1": LibraryRootSynchronizationStatus(
        rootId: "root-1",
        rootGeneration: generation ?? BigInt.one,
        availability: LibraryRootAvailability.available,
        freshness: LibraryCatalogFreshness.updating,
        freshnessCause: LibraryCatalogFreshnessCause.evidenceGap,
        phase: LibrarySynchronizationPhase.inventoryEnumeration,
        phaseStartedAt: DateTime.utc(2026, 8, 21),
        sourceStatus: LibraryChangeSourceStatus.healthy,
        pendingChangeCount: BigInt.zero,
        retryWaitCount: BigInt.zero,
        freshnessUnknownCount: BigInt.one,
        lastIssueCode: "change_source_rescan_required",
      ),
    },
  );
}

LibrarySynchronizationSnapshot _persistenceFailureSnapshot() {
  return LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: BigInt.one,
    appliedMutationCount: 0,
    roots: {
      "root-1": LibraryRootSynchronizationStatus(
        rootId: "root-1",
        rootGeneration: BigInt.one,
        availability: LibraryRootAvailability.available,
        freshness: LibraryCatalogFreshness.needsReconciliation,
        freshnessCause: LibraryCatalogFreshnessCause.pendingChanges,
        phase: LibrarySynchronizationPhase.blocked,
        phaseStartedAt: DateTime.utc(2026, 8, 21),
        sourceStatus: LibraryChangeSourceStatus.healthy,
        pendingChangeCount: BigInt.one,
        retryWaitCount: BigInt.one,
        freshnessUnknownCount: BigInt.zero,
        lastIssueCode: "catalog_database_busy",
      ),
    },
  );
}

LibrarySynchronizationSnapshot _synchronizedRootSnapshot() {
  return LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: BigInt.two,
    appliedMutationCount: 1,
    roots: {
      "root-1": LibraryRootSynchronizationStatus(
        rootId: "root-1",
        rootGeneration: BigInt.one,
        availability: LibraryRootAvailability.available,
        freshness: LibraryCatalogFreshness.synchronized,
        freshnessCause: LibraryCatalogFreshnessCause.noPendingChanges,
        phase: LibrarySynchronizationPhase.synchronized,
        phaseStartedAt: DateTime.utc(2026, 8, 21),
        sourceStatus: LibraryChangeSourceStatus.healthy,
        pendingChangeCount: BigInt.zero,
        retryWaitCount: BigInt.zero,
        freshnessUnknownCount: BigInt.zero,
      ),
    },
  );
}

LibrarySnapshot _snapshotFromState(LibraryState state, {BigInt? revision}) {
  return LibrarySnapshot(
    catalogPath: state.catalogPath ?? "",
    revision: revision ?? state.catalogRevision ?? BigInt.one,
    queryId: state.queryId,
    roots: state.roots,
    assets: state.assets,
  );
}

class _TestLibrarySynchronization implements LibrarySynchronization {
  _TestLibrarySynchronization(this._current);

  final StreamController<LibrarySynchronizationSnapshot> _updates =
      StreamController.broadcast(sync: true);
  LibrarySynchronizationSnapshot _current;

  @override
  LibrarySynchronizationSnapshot get current => _current;

  @override
  Future<void> start() async {}

  @override
  Future<void> stop() async {}

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => _updates.stream;

  void publish(LibrarySynchronizationSnapshot snapshot) {
    _current = snapshot;
    _updates.add(snapshot);
  }

  Future<void> dispose() => _updates.close();
}

class _SynchronizationViewerCatalog
    implements
        LibraryCatalog,
        LibraryStableQueryAnchorCatalog,
        LibraryStableAssetCatalog {
  _SynchronizationViewerCatalog(this.snapshot);

  LibrarySnapshot snapshot;
  final List<String> requestedLocationIds = [];
  final List<String?> preferredLocationIds = [];
  Completer<LibraryAsset?>? _heldAssetLookup;

  @override
  Future<LibrarySnapshot> loadAroundAsset({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String requestedLocationId,
    required String anchorAssetId,
    required int fallbackGlobalItemIndex,
  }) async {
    requestedLocationIds.add(requestedLocationId);
    return snapshot;
  }

  @override
  Future<LibraryAsset?> loadAssetById({
    required String assetId,
    String? preferredLocationId,
  }) async {
    preferredLocationIds.add(preferredLocationId);
    final heldAssetLookup = _heldAssetLookup;
    if (heldAssetLookup != null) {
      return heldAssetLookup.future;
    }
    for (final asset in snapshot.assets) {
      if (asset.assetId == assetId) {
        return asset;
      }
    }
    return null;
  }

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
          aspectRatioSum: snapshot.assets.length * 4 / 3,
        ),
      ],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) async => true;

  void holdNextAssetLookup() {
    _heldAssetLookup = Completer<LibraryAsset?>();
  }

  void completeHeldAssetLookup(LibraryAsset? asset) {
    final heldAssetLookup = _heldAssetLookup;
    _heldAssetLookup = null;
    heldAssetLookup?.complete(asset);
  }
}

class _FailingSynchronizationCatalog implements LibraryCatalog {
  int loadCount = 0;

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    loadCount += 1;
    throw const LibraryCatalogFailure(
      code: "catalog_unavailable",
      message: "controlled synchronization refresh failure",
    );
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) => throw UnimplementedError();

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    return LibraryTimeline(
      revision: BigInt.one,
      queryId: "viewer-position",
      totalItems: 1,
      buckets: const [LibraryTimeBucket(itemCount: 1, aspectRatioSum: 4 / 3)],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) => throw UnimplementedError();
}

class _HeldFailureSynchronizationCatalog implements LibraryCatalog {
  _HeldFailureSynchronizationCatalog(this.snapshot);

  final LibrarySnapshot snapshot;
  final Completer<void> _heldLoad = Completer<void>();
  int loadCount = 0;

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    loadCount += 1;
    if (loadCount == 1) {
      await _heldLoad.future;
      throw const LibraryCatalogFailure(
        code: "catalog_unavailable",
        message: "controlled in-flight synchronization failure",
      );
    }
    return snapshot;
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async => snapshot;

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      _timelineFromSnapshot(snapshot);

  @override
  Future<bool> unregisterRoot(String rootId) async => true;

  void failHeldLoad() => _heldLoad.complete();
}

class _FailOnceSynchronizationCatalog implements LibraryCatalog {
  _FailOnceSynchronizationCatalog(this.snapshot);

  final LibrarySnapshot snapshot;
  int loadCount = 0;

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    loadCount += 1;
    if (loadCount == 1) {
      throw const LibraryCatalogFailure(
        code: "catalog_unavailable",
        message: "controlled synchronization refresh failure",
      );
    }
    return snapshot;
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async => snapshot;

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      _timelineFromSnapshot(snapshot);

  @override
  Future<bool> unregisterRoot(String rootId) async => true;
}

class _HeldLibraryScanner implements LibraryScanner {
  final StreamController<LibraryScanUpdate> _updates =
      StreamController.broadcast(sync: true);
  String? scanId;
  int scanCount = 0;

  @override
  bool cancel(String scanId) => true;

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;

  @override
  bool pause(String scanId) => true;

  @override
  bool suspend(String scanId) => true;

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    scanCount += 1;
    this.scanId = scanId;
    return _updates.stream;
  }

  @override
  Stream<LibraryScanUpdate> resume({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    scanCount += 1;
    this.scanId = scanId;
    return _updates.stream;
  }

  void add(LibraryScanUpdate update) => _updates.add(update);

  Future<void> close() => _updates.close();

  Future<void> dispose() async {
    if (!_updates.isClosed) {
      await _updates.close();
    }
  }
}

class _FailingDirectoryPicker implements DirectoryPicker {
  const _FailingDirectoryPicker();

  @override
  Future<String?> pickDirectory() {
    throw StateError("controlled picker failure");
  }
}

LibraryTimeline _timelineFromSnapshot(LibrarySnapshot snapshot) {
  return LibraryTimeline(
    revision: snapshot.revision,
    queryId: snapshot.queryId,
    totalItems: snapshot.assets.length,
    buckets: [
      LibraryTimeBucket(
        itemCount: snapshot.assets.length,
        aspectRatioSum: snapshot.assets.length * 4 / 3,
      ),
    ],
  );
}

class _RecordingPlatformActions implements LibraryPlatformActions {
  final List<String> revealedFiles = [];

  @override
  Future<void> copyText(String value) async {}

  @override
  Future<void> revealDirectory(String path) async {}

  @override
  Future<void> revealFile(String path) async {
    revealedFiles.add(path);
  }

  @override
  Future<void> revealLibraryFolder(
    String rootPath,
    String relativePath,
  ) async {}
}
