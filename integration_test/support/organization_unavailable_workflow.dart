import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_image.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_image.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

import "source_read_block.dart";
import "viewer_source_workflow.dart";

Future<void> verifyOrganizationUnavailableWorkflow(WidgetTester tester) async {
  await expectLater(SourceReadBlock.acquire("README.md"), throwsArgumentError);
  await _mode(tester, "整理模式");
  expect(find.byKey(const Key("library-review-toolbar")), findsOneWidget);
  expect(find.bySemanticsLabel("待整理 2"), findsOneWidget);
  final tile = find.byType(LibraryPhotoTile).hitTestable().first;
  final selected = tester.widget<LibraryPhotoTile>(tile).asset;
  final target = LibraryOrganizationTarget.fromAsset(selected);
  final block = await SourceReadBlock.acquire(selected.sourcePath);
  try {
    await LibrarySourceImage(selected).evict();
    await tester.tapAt(tester.getRect(tile).topLeft + const Offset(16, 16));
    await _waitForFallback(tester);
    expect(_current(tester).locationId, selected.locationId);
    await _expectPending(target);

    await tester.tap(find.byKey(const Key("viewer-next")));
    await tester.pump();
    await waitForViewerOriginal(tester, differentFrom: selected.locationId);
    final next = _current(tester);
    await _expectPending(target);
    await _expectPending(LibraryOrganizationTarget.fromAsset(next));

    await tester.tap(find.byKey(const Key("viewer-previous")));
    await tester.pump();
    await _waitForFallback(tester);
    expect(_current(tester).locationId, selected.locationId);
    await _expectPending(target);
    await block.release();
    await tester.tap(
      find.descendant(
        of: find.byType(LibraryViewerImage),
        matching: find.byKey(const Key("viewer-original-retry")),
      ),
    );
    await waitForViewerOriginal(tester);
    expect(_current(tester).locationId, selected.locationId);
    await _expectPending(target);
  } finally {
    await block.release();
  }
  await tester.tap(find.byKey(const Key("viewer-back-button")));
  await tester.pumpAndSettle();
  expect(find.bySemanticsLabel("待整理 2"), findsOneWidget);
  await _mode(tester, "退出整理");
  expect(find.byKey(const Key("library-review-toolbar")), findsNothing);
  expect(tester.takeException(), isNull);
  debugPrint("AME_ORGANIZATION_UNAVAILABLE_WORKFLOW_PASSED");
}

Future<void> _expectPending(LibraryOrganizationTarget target) async {
  final decision = (await const RustLibraryOrganization().inspect([
    target,
  ])).decisions.single;
  expect(decision.disposition, LibraryOrganizationDisposition.pending);
  expect(decision.album, isNull);
  expect(decision.favorite, isFalse);
}

Future<void> _mode(WidgetTester tester, String label) async {
  await tester.tap(find.byKey(const Key("library-more-menu")));
  await tester.pumpAndSettle();
  await tester.tap(
    find.descendant(
      of: find.byWidgetPredicate((widget) => widget is PopupMenuItem<Object?>),
      matching: find.text(label),
    ),
  );
  await tester.pumpAndSettle();
}

LibraryAsset _current(WidgetTester tester) =>
    tester.widget<LibraryViewerImage>(find.byType(LibraryViewerImage)).asset;

Future<void> _waitForFallback(WidgetTester tester) async {
  final deadline = DateTime.now().add(const Duration(seconds: 15));
  while (find
      .text(LibraryStrings.originalImagePreviewFallback)
      .evaluate()
      .isEmpty) {
    if (DateTime.now().isAfter(deadline)) {
      throw TestFailure("The unavailable source did not retain its fallback");
    }
    await tester.pump(const Duration(milliseconds: 25));
  }
  await tester.pumpAndSettle();
}
