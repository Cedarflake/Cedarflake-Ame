import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  testWidgets("uncommitted removal failure does not interrupt another search", (
    tester,
  ) async {
    final fixture = await _show(tester);
    await fixture.controller.cancelScan();
    fixture.catalog.unregisterFailure = StateError("unregister unavailable");
    await fixture.controller.unregisterRoot(otherPublishedRoot);
    await tester.pump();
    expect(fixture.state.status, LibraryStatus.failed);
    expect(fixture.state.taskKind, LibraryTaskKind.remove);
    expect(fixture.state.isRemovalCommitted, isFalse);
    final pending = fixture.catalog.pendingLoad = Completer<LibrarySnapshot>();
    await tester.enterText(find.byKey(const Key("library-search")), "2");
    final input = tester.widget<EditableText>(find.byType(EditableText));
    await tester.pump(const Duration(milliseconds: 300));
    expect(fixture.state.isRefreshingQuery, isTrue);
    expect(input.focusNode.hasFocus, isTrue);
    tester.testTextInput.enterText("20");
    await tester.pump(const Duration(milliseconds: 300));
    await tester.pump();
    expect(fixture.state.query.searchText, "20");
    pending.complete(
      fixture.catalog.snapshot(const LibraryGalleryQuery(searchText: "2")),
    );
    await tester.pump();
    await tester.pump();
    expect(fixture.state.query.searchText, "20");
    expect(input.focusNode.hasFocus, isTrue);
    expect(fixture.catalog.removedIds, isEmpty);
  });

  testWidgets(
    "search keeps focus while a newer query supersedes a pending read",
    (tester) async {
      final fixture = await _show(tester);
      final oldQuery = fixture.catalog.pendingLoad =
          Completer<LibrarySnapshot>();
      await tester.enterText(find.byKey(const Key("library-search")), "2");
      final input = tester.widget<EditableText>(find.byType(EditableText));
      expect(input.focusNode.hasFocus, isTrue);
      await tester.pump(const Duration(milliseconds: 300));
      expect(fixture.state.isRefreshingQuery, isTrue);
      expect(input.focusNode.hasFocus, isTrue);

      final newQuery = fixture.catalog.pendingLoad =
          Completer<LibrarySnapshot>();
      tester.testTextInput.enterText("20");
      await tester.pump(const Duration(milliseconds: 300));
      expect(
        (fixture.state.queryActivity as LibraryQueryLoading)
            .requestedQuery
            .searchText,
        "20",
      );
      expect(input.focusNode.hasFocus, isTrue);
      newQuery.complete(
        fixture.catalog.snapshot(const LibraryGalleryQuery(searchText: "20")),
      );
      await tester.pump();
      await tester.pump();
      expect(fixture.state.query.searchText, "20");
      expect(input.focusNode.hasFocus, isTrue);

      oldQuery.complete(
        fixture.catalog.snapshot(const LibraryGalleryQuery(searchText: "2")),
      );
      await tester.pump();
      await tester.pump();
      expect(fixture.state.query.searchText, "20");
      expect(input.focusNode.hasFocus, isTrue);
      tester.testTextInput.enterText("200");
      await tester.pump(const Duration(milliseconds: 300));
      await tester.pump();
      expect(fixture.state.query.searchText, "200");
      expect(input.focusNode.hasFocus, isTrue);
      expect(fixture.scanner.startedRoots, isEmpty);
    },
  );

  testWidgets("failed search leaves focus ready for corrected text", (
    tester,
  ) async {
    final fixture = await _show(tester);
    final pending = fixture.catalog.pendingLoad = Completer<LibrarySnapshot>();
    await tester.enterText(find.byKey(const Key("library-search")), "bad");
    final input = tester.widget<EditableText>(find.byType(EditableText));
    await tester.pump(const Duration(milliseconds: 300));
    pending.completeError(StateError("query unavailable"));
    await tester.pump();
    await tester.pump();
    expect(fixture.state.queryActivity, isA<LibraryQueryFailed>());
    expect(input.focusNode.hasFocus, isTrue);
    tester.testTextInput.enterText("good");
    await tester.pump(const Duration(milliseconds: 300));
    await tester.pump();
    expect(fixture.state.query.searchText, "good");
    expect(input.focusNode.hasFocus, isTrue);
    expect(tester.takeException(), isNull);
  });
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
