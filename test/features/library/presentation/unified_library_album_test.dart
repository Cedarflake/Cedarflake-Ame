import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/app/presentation/ame_notifications.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_history.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";
import "support/library_screen_fixture.dart";

void main() {
  testWidgets(
    "album navigation, rename, deletion and undo use the existing gallery",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final state = populatedScreenState(totalItems: 1);
      final catalog = RecordingQueryCatalog(
        LibrarySnapshot(
          catalogPath: state.catalogPath ?? "",
          revision: state.catalogRevision ?? BigInt.zero,
          queryId: "album-query",
          roots: state.roots,
          assets: state.assets,
        ),
      );
      final service = OrganizationServiceFixture()
        ..albumItems = [const LibraryOrganizationAlbum("a", "Landscape")];
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryOrganizationProvider.overrideWithValue(service),
          libraryScannerProvider.overrideWithValue(const NoopLibraryScanner()),
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey("source-title-root-1")));
      await tester.pumpAndSettle();
      expect(catalog.loadQueries.last.rootId, "root-1");
      final albumTile = find.byKey(const ValueKey("library-sidebar-album-a"));
      expect(albumTile, findsNothing);
      await tester.tap(find.byKey(const Key("workbench-albums")));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("library-navigation")), findsNothing);
      expect(find.text("选择相册"), findsOneWidget);
      await tester.tap(albumTile);
      await tester.pumpAndSettle();
      expect(
        catalog.loadQueries.last.collection,
        const LibraryGalleryCollection.album("a"),
      );
      expect(catalog.loadQueries.last.rootId, isNull);
      expect(catalog.loadQueries.last.folderRelativePath, isNull);
      expect(tester.widget<ListTile>(albumTile).selected, isTrue);
      await tester.tap(find.byKey(const Key("workbench-gallery")));
      await tester.pumpAndSettle();
      expect(albumTile, findsNothing);
      expect(catalog.loadQueries.last.rootId, "root-1");
      expect(
        catalog.loadQueries.last.collection,
        const LibraryGalleryCollection.library(),
      );
      await tester.tap(find.byKey(const Key("workbench-albums")));
      await tester.pumpAndSettle();
      expect(tester.widget<ListTile>(albumTile).selected, isTrue);
      expect(
        catalog.loadQueries.last.collection,
        const LibraryGalleryCollection.album("a"),
      );

      Future<void> openMenu() async {
        final menu = find.byKey(const ValueKey("library-sidebar-album-menu-a"));
        await tester.pumpAndSettle();
        await tester.tap(menu);
        await tester.pumpAndSettle();
      }

      await openMenu();
      await tester.tap(find.text("重命名"));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const Key("library-album-name")),
        "Renamed",
      );
      await tester.tap(find.byKey(const Key("library-album-save")));
      await tester.pumpAndSettle();
      expect(find.text("Renamed"), findsNWidgets(2));
      expect(find.byType(AmeNotificationSurface), findsOneWidget);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );

      await openMenu();
      await tester.tap(find.text("删除相册"));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("library-album-save")));
      await tester.pumpAndSettle();
      expect(albumTile, findsNothing);
      expect(
        catalog.loadQueries.last.collection,
        const LibraryGalleryCollection.library(),
      );
      expect(find.text("已删除相册"), findsOneWidget);
      expect(find.text("选择相册"), findsOneWidget);
      final deletionId = service.albumWrites.last.id;
      service.undoResult = (id) async {
        service.albumItems = [const LibraryOrganizationAlbum("a", "Renamed")];
        return LibraryOrganizationHistoryReceipt(
          operation: service.receipt(id),
        );
      };
      await tester.tap(find.byKey(const Key("notification-primary-action")));
      await tester.pumpAndSettle();
      expect(service.undos.single.originalId, deletionId);
      expect(albumTile, findsOneWidget);
      expect(
        container.read(ameNotificationControllerProvider).hasUnread,
        isFalse,
      );
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      await tester.pump(const Duration(seconds: 6));
      await tester.pumpAndSettle();
      expect(find.byType(AmeNotificationSurface), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );
}
