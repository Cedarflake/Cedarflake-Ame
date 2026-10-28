import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_update_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

import "support/library_screen_fixture.dart";

void main() {
  testWidgets("shows scan controls in temporary import feedback", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            const LibraryState(
              status: LibraryStatus.scanning,
              scanId: "scan-1",
              rootPath: "C:\\Pictures",
              displayRootPath: "C:\\Pictures",
              visitedEntries: 128,
              stagedAssetCount: 64,
            ),
          ),
        ],
        child: const AmeApp(),
      ),
    );

    expect(find.text("正在添加“Pictures”…"), findsOneWidget);
    expect(find.text("已检查 128 个文件 · 已找到 64 张图片"), findsOneWidget);
    expect(find.byKey(const Key("library-pause-button")), findsOneWidget);
    expect(find.byKey(const Key("library-cancel-button")), findsOneWidget);
    expect(find.byKey(const Key("library-task-activity-button")), findsNothing);
  });

  testWidgets("shows determinate progress while validating staged images", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            const LibraryState(
              status: LibraryStatus.scanning,
              scanId: "scan-1",
              rootPath: "C:\\Pictures",
              displayRootPath: "C:\\Pictures",
              visitedEntries: 50304,
              stagedAssetCount: 48384,
              scanPhase: LibraryScanPhase.finalizing,
              validatedAssetCount: 128,
              validationAssetCount: 48384,
            ),
          ),
        ],
        child: const AmeApp(),
      ),
    );

    expect(find.text("正在检查“Pictures”的图片…"), findsOneWidget);
    expect(find.text("已检查 128 / 48384 张图片 · 已检查 50304 个文件"), findsOneWidget);
    final progress = tester.widget<LinearProgressIndicator>(
      find.descendant(
        of: find.byKey(const Key("library-task-surface")),
        matching: find.byType(LinearProgressIndicator),
      ),
    );
    expect(progress.value, closeTo(128 / 48384, 0.000001));
  });

  testWidgets("keeps completed import feedback until it is acknowledged", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final state = populatedScreenState(totalItems: 64).copyWith(
      scanId: "scan-completed",
      rootPath: "C:\\Pictures",
      displayRootPath: "C:\\Pictures",
      visitedEntries: 128,
      stagedAssetCount: 64,
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryScannerProvider.overrideWithValue(const NoopLibraryScanner()),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final container = ProviderScope.containerOf(
      tester.element(find.byType(AmeApp)),
    );
    expect(container.read(libraryControllerProvider).scanId, "scan-completed");
    expect(find.byKey(const Key("library-task-surface")), findsOneWidget);
    expect(find.text("添加完成"), findsOneWidget);
    expect(find.text("已添加 64 张图片"), findsOneWidget);
    expect(
      find.byKey(const Key("library-task-dismiss-button")),
      findsOneWidget,
    );
    final taskSurface = tester.widget<Material>(
      find
          .descendant(
            of: find.byKey(const Key("library-task-surface")),
            matching: find.byType(Material),
          )
          .first,
    );
    final theme = Theme.of(
      tester.element(find.byKey(const Key("library-task-surface"))),
    );
    expect(taskSurface.color, theme.snackBarTheme.backgroundColor);
    expect(taskSurface.elevation, theme.snackBarTheme.elevation);
    expect(
      tester.getSize(find.byKey(const Key("library-task-surface"))).width,
      theme.snackBarTheme.width,
    );

    await tester.tap(find.byKey(const Key("library-task-dismiss-button")));
    await tester.pump();

    expect(find.text("添加完成"), findsNothing);
    expect(find.byKey(const Key("library-task-dismiss-button")), findsNothing);
  });

  testWidgets("active main task takes priority over terminal update feedback", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final populated = populatedScreenState(totalItems: 1);
    final root = populated.roots.single;
    final state = populated.copyWith(
      status: LibraryStatus.scanning,
      scanId: "scan-import",
      rootPath: "C:\\NewPictures",
      displayRootPath: "C:\\NewPictures",
      taskKind: LibraryTaskKind.import,
      visitedEntries: 12,
      stagedAssetCount: 4,
    );
    final terminalUpdates = LibraryUpdateState(
      tasksByRootId: {
        root.id: LibraryRootUpdateTask(
          root: root,
          scanId: "scan-update",
          phase: LibraryRootUpdatePhase.completed,
          acceptedItems: 1,
        ),
      },
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryScannerProvider.overrideWithValue(const NoopLibraryScanner()),
          libraryUpdateControllerProvider.overrideWith(
            () => _FixedLibraryUpdateController(terminalUpdates),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    expect(find.byKey(const Key("library-task-surface")), findsOneWidget);
    expect(find.text("正在添加“NewPictures”…"), findsOneWidget);
    expect(find.byKey(const Key("library-update-task-surface")), findsNothing);
  });

  testWidgets("root removal keeps another root update and its cancel action", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final populated = populatedScreenState(totalItems: 1);
    final updatingRoot = populated.roots.single;
    const removingRoot = LibraryRoot(
      id: "root-b",
      path: "D:\\Archive",
      displayPath: "D:\\Archive",
      activeScanId: "scan-b",
      createdUnixMs: 2,
      assetCount: 1,
      issueCount: 0,
      availability: LibraryRootAvailability.available,
    );
    final state = populated.copyWith(
      roots: [...populated.roots, removingRoot],
      status: LibraryStatus.removing,
      taskKind: LibraryTaskKind.remove,
      removingRootId: removingRoot.id,
      removingRootDisplayPath: removingRoot.displayPath,
    );
    final activeUpdates = LibraryUpdateState(
      tasksByRootId: {
        updatingRoot.id: LibraryRootUpdateTask(
          root: updatingRoot,
          scanId: "scan-update-a",
          phase: LibraryRootUpdatePhase.discovering,
          visitedEntries: 12,
          acceptedItems: 1,
        ),
      },
    );
    final updateController = _FixedLibraryUpdateController(activeUpdates);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryScannerProvider.overrideWithValue(const NoopLibraryScanner()),
          libraryUpdateControllerProvider.overrideWith(() => updateController),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    expect(find.byKey(const Key("library-task-surface")), findsOneWidget);
    expect(
      find.byKey(const Key("library-update-task-surface")),
      findsOneWidget,
    );
    final updateCancel = find.descendant(
      of: find.byKey(Key("library-update-task-${updatingRoot.id}")),
      matching: find.text("取消"),
    );
    expect(updateCancel, findsOneWidget);

    await tester.tap(updateCancel);
    await tester.pump();
    expect(updateController.cancelledRootId, updatingRoot.id);
  });

  testWidgets("aligns a pending source with existing source rows", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final state = populatedScreenState(totalItems: 1).copyWith(
      status: LibraryStatus.scanning,
      scanId: "scan-pending",
      rootPath: "C:\\Documents",
      displayRootPath: "C:\\Documents",
      visitedEntries: 12,
      stagedAssetCount: 3,
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [initialLibraryStateProvider.overrideWithValue(state)],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final sourceIcon = find.byKey(const ValueKey("source-icon-root-1"));
    final pendingIcon = find.byKey(const Key("pending-source-icon"));
    final sourceTitle = find.byKey(const ValueKey("source-title-root-1"));
    final pendingTitle = find.byKey(const Key("pending-source-title"));
    expect(sourceIcon, findsOneWidget);
    expect(pendingIcon, findsOneWidget);
    expect(find.byKey(const Key("pending-source-progress")), findsOneWidget);
    expect(
      tester.getTopLeft(sourceIcon).dx,
      closeTo(tester.getTopLeft(pendingIcon).dx, 0.1),
    );
    expect(
      tester.getTopLeft(sourceTitle).dx,
      closeTo(tester.getTopLeft(pendingTitle).dx, 0.1),
    );
  });

  testWidgets("keeps a missing source visible with its cached catalog state", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: "query-1",
      roots: const [
        LibraryRoot(
          id: "missing-root",
          path: "E:\\DisconnectedPictures",
          displayPath: "E:\\DisconnectedPictures",
          activeScanId: "scan-1",
          createdUnixMs: 1,
          assetCount: 42,
          issueCount: 0,
          availability: LibraryRootAvailability.missing,
          availabilityMessage: "The source volume is disconnected",
        ),
      ],
      assets: const [],
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(snapshot),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    expect(find.text("DisconnectedPictures"), findsOneWidget);
    expect(find.text("目录不存在"), findsOneWidget);
    expect(find.byIcon(Symbols.folder_off_rounded), findsOneWidget);
  });
}

class _FixedLibraryUpdateController extends LibraryUpdateController {
  _FixedLibraryUpdateController(this.initialState);

  final LibraryUpdateState initialState;
  String? cancelledRootId;

  @override
  LibraryUpdateState build() => initialState;

  @override
  void cancel(String rootId) {
    cancelledRootId = rootId;
  }
}
