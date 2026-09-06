import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_navigation.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_task_surface.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  testWidgets(
    "retained first import permits root navigation, search and settings return",
    (tester) async {
      final fixture = await _show(tester);
      expect(fixture.state.status, LibraryStatus.paused);
      expect(find.byKey(const Key("library-resume-button")), findsOneWidget);
      expect(find.byKey(const Key("library-cancel-button")), findsOneWidget);
      expect(
        tester
            .widget<SearchBar>(find.byKey(const Key("library-search")))
            .enabled,
        isTrue,
      );
      final navigation = tester.widget<LibraryNavigation>(
        find.byType(LibraryNavigation),
      );
      expect(navigation.isBusy, isFalse);
      expect(navigation.isAddingSourceDisabled, isTrue);
      expect(navigation.addingSourceDisabledReason, "请先继续或取消已暂停的任务");

      await tester.tap(find.byKey(const Key("library-sidebar-settings")));
      await tester.pump();
      expect(
        tester
            .widget<LibraryNavigation>(find.byType(LibraryNavigation))
            .isSettingsSelected,
        isTrue,
      );
      await tester.tap(find.byKey(const Key("library-sidebar-library")));
      await tester.pump();
      expect(
        tester
            .widget<LibraryNavigation>(find.byType(LibraryNavigation))
            .isSettingsSelected,
        isFalse,
      );
      await tester.tap(find.byKey(const ValueKey("source-title-other")));
      await tester.pump();
      await tester.pump();
      expect(fixture.state.query.rootId, "other");
      expect(fixture.state.scanId, retainedScanCheckpoint.scanId);
      expect(fixture.scanner.startedRoots, isEmpty);
    },
  );

  testWidgets(
    "paused Cancel gives immediate feedback and only clears after commit",
    (tester) async {
      final fixture = await _show(tester);
      final commit = fixture.scanner.pendingCancellation = Completer();
      await tester.tap(find.byKey(const Key("library-cancel-button")));
      await tester.pump();
      expect(fixture.state.status, LibraryStatus.discarding);
      expect(find.text("正在取消…"), findsOneWidget);
      expect(
        find.descendant(
          of: find.byType(LibraryTaskSurface),
          matching: find.byType(LinearProgressIndicator),
        ),
        findsOneWidget,
      );
      expect(
        tester
            .widget<TextButton>(find.widgetWithText(TextButton, "继续"))
            .onPressed,
        isNull,
      );
      expect(fixture.scanner.cancelRetainedIds, [
        retainedScanCheckpoint.scanId,
      ]);
      expect(fixture.scanner.startedRoots, isEmpty);
      commit.complete();
      await tester.pump();
      expect(find.byType(LibraryTaskSurface), findsNothing);
      expect(
        tester
            .widget<LibraryNavigation>(find.byType(LibraryNavigation))
            .isAddingSourceDisabled,
        isFalse,
      );
      expect(fixture.scanner.cancelActiveIds, isEmpty);
    },
  );

  testWidgets(
    "cancel failure retains Continue and Cancel without auto execution",
    (tester) async {
      final fixture = await _show(tester);
      fixture.scanner.cancelFailure = StateError("catalog write blocked");
      await tester.tap(find.byKey(const Key("library-cancel-button")));
      await tester.pump();
      expect(find.textContaining("catalog write blocked"), findsOneWidget);
      expect(find.byKey(const Key("library-resume-button")), findsOneWidget);
      expect(find.byKey(const Key("library-cancel-button")), findsOneWidget);
      expect(fixture.state.status, LibraryStatus.paused);
      expect(fixture.scanner.startedRoots, isEmpty);
    },
  );

  testWidgets(
    "paused intent and another root update keep independent controls",
    (tester) async {
      final fixture = await _show(tester);
      await _startOtherUpdate(tester);
      expect(fixture.scanner.startedRoots, ["C:\\Other"]);
      final updateSurface = find.byKey(
        const Key("library-update-task-surface"),
      );
      expect(updateSurface, findsOneWidget);
      expect(find.byKey(const Key("library-resume-button")), findsOneWidget);
      expect(find.byKey(const Key("library-cancel-button")), findsOneWidget);
      expect(
        find.descendant(
          of: updateSurface,
          matching: find.widgetWithText(TextButton, "取消"),
        ),
        findsOneWidget,
      );
      await tester.tap(find.byKey(const Key("library-resume-button")));
      await tester.pump();
      expect(fixture.state.status, LibraryStatus.paused);
      expect(fixture.state.scanId, retainedScanCheckpoint.scanId);
      expect(find.textContaining("其他图库更新正在运行"), findsOneWidget);
      expect(fixture.scanner.resumedIds, isEmpty);
      expect(tester.takeException(), isNull);

      await tester.tap(find.byKey(const Key("library-sidebar-settings")));
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey("source-title-other")));
      await tester.pump();
      await tester.pump();
      expect(fixture.state.query.rootId, "other");
      expect(
        tester
            .widget<LibraryNavigation>(find.byType(LibraryNavigation))
            .isSettingsSelected,
        isFalse,
      );
      fixture.scanner.failUpdates();
      await tester.pump();
      await tester.pump();
      expect(find.textContaining("更新夹具失败"), findsOneWidget);
      final retry = find.descendant(
        of: updateSurface,
        matching: find.widgetWithText(TextButton, "重试"),
      );
      expect(retry, findsOneWidget);
      await tester.tap(retry);
      await tester.pump();
      expect(fixture.scanner.startedRoots, ["C:\\Other", "C:\\Other"]);
      expect(fixture.state.scanId, retainedScanCheckpoint.scanId);
      final cancelUpdate = find.descendant(
        of: updateSurface,
        matching: find.widgetWithText(TextButton, "取消"),
      );
      await tester.tap(cancelUpdate);
      await tester.pump();
      expect(fixture.scanner.cancelActiveIds, hasLength(1));
      fixture.scanner.finishUpdates();
      await tester.pump();
    },
  );

  testWidgets("query failure exposes read-only retry beside a paused task", (
    tester,
  ) async {
    final fixture = await _show(tester);
    fixture.catalog.loadFailure = StateError("catalog query blocked");
    await tester.tap(find.byKey(const ValueKey("source-title-other")));
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const Key("library-query-failure")), findsOneWidget);
    expect(find.textContaining("catalog query blocked"), findsOneWidget);
    expect(find.byKey(const Key("library-resume-button")), findsOneWidget);
    expect(find.byKey(const Key("library-cancel-button")), findsOneWidget);
    fixture.catalog.loadFailure = null;
    await tester.tap(find.byKey(const Key("library-query-retry")));
    await tester.pump();
    await tester.pump();
    expect(fixture.state.query.rootId, "other");
    expect(find.byKey(const Key("library-query-failure")), findsNothing);
    expect(fixture.state.scanId, retainedScanCheckpoint.scanId);
    expect(fixture.scanner.startedRoots, isEmpty);
  });

  testWidgets("query loading cannot replace another root update surface", (
    tester,
  ) async {
    final fixture = await _show(tester);
    await fixture.controller.cancelScan();
    await tester.pump();
    await _startOtherUpdate(tester);
    final pending = fixture.catalog.pendingLoad = Completer();
    await tester.tap(find.byKey(const ValueKey("source-title-other")));
    await tester.pump();
    expect(fixture.state.isRefreshingQuery, isTrue);
    expect(find.byKey(const Key("library-top-loading")), findsOneWidget);
    expect(find.byType(LibraryTaskSurface), findsNothing);
    final updateSurface = find.byKey(const Key("library-update-task-surface"));
    expect(updateSurface, findsOneWidget);
    expect(
      find.descendant(
        of: updateSurface,
        matching: find.widgetWithText(TextButton, "取消"),
      ),
      findsOneWidget,
    );
    pending.complete(
      fixture.catalog.snapshot(const LibraryGalleryQuery(rootId: "other")),
    );
    await tester.pump();
    await tester.pump();
    expect(fixture.state.query.rootId, "other");
    expect(fixture.state.isRefreshingQuery, isFalse);
    expect(find.byType(LibraryTaskSurface), findsNothing);
    expect(updateSurface, findsOneWidget);
    expect(fixture.scanner.startedRoots, ["C:\\Other"]);
  });

  testWidgets(
    "published primary update exposes Cancel but not unsupported Pause",
    (tester) async {
      final fixture = await _show(tester);
      await fixture.controller.cancelScan();
      await fixture.controller.scanDirectory("C:\\Other");
      await tester.pump();
      expect(fixture.state.taskKind, LibraryTaskKind.update);
      expect(find.byKey(const Key("library-pause-button")), findsNothing);
      expect(find.byKey(const Key("library-cancel-button")), findsOneWidget);
    },
  );

  testWidgets(
    "failed primary task stays actionable while another root updates",
    (tester) async {
      final fixture = await _show(tester);
      await fixture.controller.cancelScan();
      await fixture.controller.scanDirectory("C:\\Retained");
      fixture.scanner.failUpdates();
      await tester.pump();
      await tester.pump();
      expect(fixture.state.status, LibraryStatus.failed);
      await _startOtherUpdate(tester);
      expect(
        find.byKey(const Key("library-update-task-surface")),
        findsOneWidget,
      );
      expect(find.byKey(const Key("library-retry-button")), findsOneWidget);
      expect(find.textContaining("更新夹具失败"), findsOneWidget);
      await tester.tap(find.byKey(const Key("library-retry-button")));
      await tester.pump();
      expect(find.textContaining("其他图库更新正在运行"), findsOneWidget);
      expect(fixture.scanner.startedRoots, ["C:\\Retained", "C:\\Other"]);
      expect(tester.takeException(), isNull);
    },
  );
}

Future<void> _startOtherUpdate(WidgetTester tester) async {
  await tester.tap(find.byKey(const ValueKey("source-more-other")));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 300));
  await tester.tap(find.text(LibraryStrings.updateLibrary));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 300));
  await tester.tap(find.byKey(const Key("library-update-confirm")));
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 300));
}

Future<RetainedScanFixture> _show(WidgetTester tester) async {
  tester.view.physicalSize = const Size(1400, 900);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  final fixture = RetainedScanFixture();
  await tester.pumpWidget(
    UncontrolledProviderScope(
      container: fixture.container,
      child: const AmeApp(),
    ),
  );
  await tester.pump();
  await tester.pump();
  return fixture;
}
