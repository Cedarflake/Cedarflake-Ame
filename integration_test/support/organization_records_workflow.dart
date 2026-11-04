import "dart:io";

import "package:cedarflake_ame/features/library/application/library_archive_controller.dart";
import "package:cedarflake_ame/features/library/application/library_archive_state.dart";
import "package:cedarflake_ame/features/library/application/library_cleanup_controller.dart";
import "package:cedarflake_ame/features/library/application/library_cleanup_state.dart";
import "package:cedarflake_ame/features/library/application/library_organization_conflicts.dart";
import "package:cedarflake_ame/features/library/application/library_sidebar_order.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_sidebar_order.dart";
import "package:cedarflake_ame/features/settings/application/ports/organization_archive_picker.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

class ControlledOrganizationArchivePicker implements OrganizationArchivePicker {
  const ControlledOrganizationArchivePicker(this.path);
  final String path;
  @override
  Future<String?> destination() async => path;
  @override
  Future<String?> backup() async => path;
}

Future<void> verifyOrganizationRecordsWorkflow(
  WidgetTester tester,
  ProviderContainer container,
  File archive,
) async {
  final conflicts = container.read(libraryOrganizationConflictsProvider)!;
  final status = await conflicts.status();
  expect(status.groups, 0);
  await expectLater(
    container
        .read(librarySidebarOrderProvider)
        .move(
          LibrarySidebarKind.sourceFolders,
          const LibrarySidebarMove(
            entryId: "missing-entry",
            anchorId: "missing-anchor",
            placement: LibrarySidebarPlacement.before,
          ),
        ),
    throwsA(isA<LibraryOrganizationFailure>()),
  );

  await _tap(tester, "library-sidebar-settings");
  await _tap(tester, "settings-archive-export");
  await _waitFor(
    tester,
    () =>
        container.read(libraryArchiveControllerProvider)
            is LibraryArchiveFinished,
  );
  expect(await archive.exists(), isTrue);
  final exported = await archive.readAsBytes();
  expect(exported, isNotEmpty);

  await _tap(tester, "settings-archive-inspect");
  await _waitFor(
    tester,
    () => find
        .byKey(const Key("organization-archive-confirmation"))
        .evaluate()
        .isNotEmpty,
  );
  final ready =
      container.read(libraryArchiveControllerProvider) as LibraryArchiveReady;
  expect(ready.inspection.duplicateChoices, greaterThanOrEqualTo(1));
  expect(ready.inspection.unresolvedPreferredCopies, 0);
  await _tap(tester, "organization-archive-restore-confirm");
  await _waitFor(
    tester,
    () =>
        container.read(libraryArchiveControllerProvider)
            is LibraryArchiveFinished,
  );
  expect(
    (container.read(libraryArchiveControllerProvider) as LibraryArchiveFinished)
        .cancelled,
    isFalse,
  );
  expect(await archive.readAsBytes(), exported);

  await _tap(tester, "settings-cleanup-inspect");
  await _waitFor(
    tester,
    () =>
        container.read(libraryCleanupControllerProvider) is LibraryCleanupReady,
  );
  final cleanup =
      container.read(libraryCleanupControllerProvider) as LibraryCleanupReady;
  expect(cleanup.inspection.total, 0);
  expect(
    tester
        .widget<FilledButton>(
          find.byKey(const Key("organization-cleanup-confirm")),
        )
        .onPressed,
    isNull,
  );
  await tester.tap(
    find.descendant(
      of: find.byKey(const Key("organization-cleanup-confirmation")),
      matching: find.widgetWithText(TextButton, "取消"),
    ),
  );
  await tester.pumpAndSettle();
  await _tap(tester, "workbench-gallery");
  expect(tester.takeException(), isNull);
  debugPrint("AME_ORGANIZATION_RECORDS_NATIVE_WORKFLOW_PASSED");
}

Future<void> _tap(WidgetTester tester, String key) async {
  final target = find.byKey(Key(key));
  await tester.ensureVisible(target);
  await tester.pumpAndSettle();
  await tester.tap(target);
  await tester.pump();
}

Future<void> _waitFor(WidgetTester tester, bool Function() condition) async {
  final deadline = DateTime.now().add(const Duration(seconds: 30));
  while (!condition()) {
    if (DateTime.now().isAfter(deadline)) {
      throw TestFailure("Organization records workflow did not complete");
    }
    await tester.pump(const Duration(milliseconds: 50));
  }
  await tester.pump();
}
