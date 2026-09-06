import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_update_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

Future<void> verifyLibraryManagementWorkflow(
  WidgetTester tester,
  ProviderContainer container,
) async {
  final before = container.read(libraryControllerProvider);
  expect(before.roots, hasLength(2));
  final first = before.roots.first;
  final second = before.roots.last;
  final previousScans = {
    for (final root in before.roots) root.id: root.activeScanId,
  };
  await _openSourceMenu(tester, first);
  await tester.tap(find.text(LibraryStrings.updateLibrary));
  await tester.pumpAndSettle();
  expect(find.byKey(const Key("library-update-dialog")), findsOneWidget);
  expect(
    tester
        .widget<CheckboxListTile>(
          find.byKey(ValueKey("library-update-root-${first.id}")),
        )
        .value,
    isTrue,
  );
  await tester.tap(find.byKey(ValueKey("library-update-root-${second.id}")));
  await tester.pump();
  expect(
    tester
        .widget<CheckboxListTile>(
          find.byKey(ValueKey("library-update-root-${second.id}")),
        )
        .value,
    isTrue,
  );
  await tester.tap(find.byKey(const Key("library-update-confirm")));
  await _pumpUntil(tester, () {
    final tasks = container.read(libraryUpdateControllerProvider).tasks;
    return tasks.length == 2 && tasks.every((task) => !task.isActive);
  });
  final tasks = container.read(libraryUpdateControllerProvider).tasks;
  expect(
    tasks.map((task) => task.phase),
    everyElement(LibraryRootUpdatePhase.completed),
    reason: tasks.map((task) => task.errorMessage).join("; "),
  );
  final updated = container.read(libraryControllerProvider);
  expect(updated.roots, hasLength(2));
  expect(updated.assets, hasLength(2));
  for (final root in updated.roots) {
    expect(root.activeScanId, isNot(previousScans[root.id]));
  }
  await tester.tap(find.byKey(const Key("library-update-dismiss-terminal")));
  await tester.pumpAndSettle();

  await _openSourceMenu(tester, first);
  await tester.tap(find.text(LibraryStrings.removeFromAme));
  await tester.pumpAndSettle();
  final confirmation = find.byType(AlertDialog);
  expect(confirmation, findsOneWidget);
  expect(find.textContaining("不会被删除或修改"), findsOneWidget);
  await tester.tap(
    find.descendant(of: confirmation, matching: find.byType(FilledButton)),
  );
  await _pumpUntil(tester, () => confirmation.evaluate().isEmpty);
  await _pumpUntil(tester, () {
    final state = container.read(libraryControllerProvider);
    return !state.isProcessing &&
        state.roots.length == 1 &&
        state.roots.single.id == second.id;
  });
  final removed = container.read(libraryControllerProvider);
  expect(removed.assets, hasLength(1));
  expect(removed.roots.single.id, second.id);
  expect(find.byKey(ValueKey("source-more-${first.id}")), findsNothing);
  await tester.tap(find.byKey(const Key("library-sidebar-settings")));
  await tester.pumpAndSettle();
  expect(find.byKey(const Key("ame-settings-page")), findsOneWidget);
  await tester.tap(find.byKey(const Key("library-sidebar-library")));
  await tester.pumpAndSettle();
  expect(find.byKey(const Key("ame-settings-page")), findsNothing);
}

Future<void> _openSourceMenu(WidgetTester tester, LibraryRoot root) async {
  await tester.tap(find.byKey(ValueKey("source-more-${root.id}")));
  await tester.pumpAndSettle();
}

Future<void> _pumpUntil(WidgetTester tester, bool Function() condition) async {
  final deadline = DateTime.now().add(const Duration(seconds: 30));
  while (!condition()) {
    if (DateTime.now().isAfter(deadline)) {
      throw TestFailure("Native library management did not settle");
    }
    await tester.pump(const Duration(milliseconds: 25));
  }
  await tester.pump();
}
