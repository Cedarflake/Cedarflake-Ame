import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_batch.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_batch_fixture.dart";
import "../support/library_organization_fixture.dart";
import "support/library_screen_fixture.dart";

void main() {
  testWidgets(
    "gallery select-all passes the complete query instead of its loaded card",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final state = populatedScreenState(totalItems: 600);
      final catalog = RecordingQueryCatalog(
        LibrarySnapshot(
          catalogPath: state.catalogPath!,
          revision: state.catalogRevision!,
          queryId: state.queryId,
          roots: state.roots,
          assets: state.assets,
        ),
      );
      final batches = OrganizationBatchesFixture();
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryOrganizationProvider.overrideWithValue(
            OrganizationServiceFixture(),
          ),
          libraryOrganizationBatchesProvider.overrideWithValue(batches),
          libraryScannerProvider.overrideWithValue(const NoopLibraryScanner()),
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("library-more-menu")));
      await tester.pumpAndSettle();
      await tester.tap(find.text("全选"));
      await tester.pumpAndSettle();
      expect(find.text("已选择 600 个项目"), findsOneWidget);
      await tester.tap(find.byKey(const Key("library-selection-favorite")));
      await tester.pumpAndSettle();
      await tester.tap(find.text("加入收藏夹"));
      await tester.pumpAndSettle();
      expect(batches.selections.single.allMatching, isTrue);
      expect(batches.selections.single.assetIds, isEmpty);
      expect(batches.selections.single.catalogRevision, state.catalogRevision);
      expect(batches.batches.single.writes, hasLength(1));
      expect(find.text("已加入收藏夹"), findsOneWidget);
      expect(find.byKey(const Key("library-selection-toolbar")), findsNothing);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      expect(tester.takeException(), isNull);
      await tester.pump(const Duration(seconds: 6));
      await tester.pumpAndSettle();
    },
  );
}
