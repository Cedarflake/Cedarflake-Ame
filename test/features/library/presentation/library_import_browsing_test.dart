import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_source_navigation_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  for (final width in [1280.0, 800.0]) {
    testWidgets(
      "existing root remains selectable during import at width $width",
      (tester) async {
        tester.view.physicalSize = Size(width, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final fixture = RetainedScanFixture();
        fixture.scanner.checkpoint = null;
        await tester.pumpWidget(
          UncontrolledProviderScope(
            container: fixture.container,
            child: const AmeApp(),
          ),
        );
        await tester.pump();
        final start = fixture.controller.scanDirectory(r"C:\Incoming");
        await tester.pump();
        await start;
        await tester.pump();
        final scan = fixture.state.primaryScanSnapshot;

        final root = find.byWidgetPredicate(
          (widget) =>
              widget is LibrarySourceNavigationTile &&
              widget.root.id == "other",
        );
        await tester.tap(
          width < 940
              ? find.descendant(of: root, matching: find.byType(IconButton))
              : find.byKey(const ValueKey("source-title-other")),
        );
        await tester.pump();
        await tester.pump();
        expect(fixture.state.query.rootId, "other");
        expect(fixture.state.primaryScanSnapshot, same(scan));
        expect(
          tester
              .widget<IconButton>(
                find.byKey(const Key("library-sidebar-import")),
              )
              .onPressed,
          isNull,
        );
        await tester.tap(find.byKey(const Key("library-sidebar-library")));
        await tester.pump();
        await tester.pump();
        expect(fixture.state.query.rootId, isNull);
        expect(fixture.state.isScanning, isTrue);
        await tester.pumpWidget(const SizedBox.shrink());
        await tester.pump();
      },
    );
  }

  testWidgets(
    "query Retry restores browsing without repeating an active import",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final fixture = RetainedScanFixture();
      fixture.scanner.checkpoint = null;
      await tester.pumpWidget(
        UncontrolledProviderScope(
          container: fixture.container,
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      final start = fixture.controller.scanDirectory(r"C:\Incoming");
      await tester.pump();
      await start;
      await tester.pump();
      final scan = fixture.state.primaryScanSnapshot;
      fixture.catalog.loadFailure = StateError("controlled read failure");
      await tester.tap(find.byKey(const ValueKey("source-title-other")));
      await tester.pump();
      await tester.pump();
      expect(fixture.state.queryActivity, isA<LibraryQueryFailed>());
      expect(fixture.state.query.rootId, isNull);
      final retry = find.byKey(const Key("library-query-retry"));
      expect(retry, findsOneWidget);
      fixture.catalog.loadFailure = null;
      final reads = fixture.catalog.firstLoads;
      await tester.tap(retry);
      await tester.pump();
      await tester.pump();
      expect(fixture.state.query.rootId, "other");
      expect(fixture.state.queryActivity, isA<LibraryQueryIdle>());
      expect(fixture.catalog.firstLoads, reads + 1);
      expect(fixture.state.primaryScanSnapshot, same(scan));
      expect(fixture.scanner.startedRoots, [r"C:\Incoming"]);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
    },
  );
}
