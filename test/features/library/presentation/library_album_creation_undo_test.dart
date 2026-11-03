import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_history.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";
import "support/library_screen_fixture.dart";

void main() {
  for (final leaveAlbum in [false, true]) {
    testWidgets(
      "creation notice undo retires the ${leaveAlbum ? 'saved' : 'active'} album before refresh",
      (tester) async {
        tester.view.physicalSize = const Size(1280, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final state = populatedScreenState(totalItems: 1);
        final catalog = RecordingQueryCatalog(
          LibrarySnapshot(
            catalogPath: state.catalogPath!,
            revision: state.catalogRevision!,
            queryId: "creation-undo",
            roots: state.roots,
            assets: state.assets,
          ),
        );
        final service = OrganizationServiceFixture();
        final container = ProviderContainer(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(state),
            libraryCatalogProvider.overrideWithValue(catalog),
            libraryOrganizationProvider.overrideWithValue(service),
            libraryScannerProvider.overrideWithValue(
              const NoopLibraryScanner(),
            ),
          ],
        );
        addTearDown(container.dispose);
        await tester.pumpWidget(
          UncontrolledProviderScope(
            container: container,
            child: const AmeApp(),
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const Key("workbench-albums")));
        await tester.pumpAndSettle();
        expect(find.text("还没有创建相册"), findsOneWidget);
        await tester.tap(find.byKey(const Key("library-sidebar-create-album")));
        await tester.pumpAndSettle();
        await tester.enterText(
          find.byKey(const Key("library-album-name")),
          "Temporary",
        );
        await tester.tap(find.byKey(const Key("library-album-save")));
        await tester.pumpAndSettle();
        final album = service.albumItems.single;
        expect(find.text("还没有创建相册"), findsNothing);
        final albumTile = find.byKey(
          ValueKey("library-sidebar-album-${album.id}"),
        );
        await tester.tap(albumTile);
        await tester.pumpAndSettle();
        expect(
          catalog.loadQueries.last.collection,
          LibraryGalleryCollection.album(album.id),
        );
        if (leaveAlbum) {
          await tester.tap(find.byKey(const Key("workbench-gallery")));
          await tester.pumpAndSettle();
        }
        service.undoResult = (id) async {
          service.albumItems = [];
          return LibraryOrganizationHistoryReceipt(
            operation: service.receipt(id),
            retiredAlbumId: album.id,
          );
        };
        catalog.loadQueries.clear();
        await tester.tap(find.byKey(const Key("notification-primary-action")));
        await tester.pumpAndSettle();
        if (leaveAlbum) {
          await tester.tap(find.byKey(const Key("workbench-albums")));
          await tester.pumpAndSettle();
        }
        expect(service.undos.single.originalId, service.albumWrites.single.id);
        expect(albumTile, findsNothing);
        expect(find.text("还没有创建相册"), findsOneWidget);
        expect(find.text("选择相册"), findsOneWidget);
        expect(catalog.loadQueries, isNotEmpty);
        expect(
          catalog.loadQueries.every(
            (query) =>
                query.collection != LibraryGalleryCollection.album(album.id),
          ),
          isTrue,
        );
        expect(find.text("图库尚未刷新，请重试。"), findsNothing);
        expect(tester.takeException(), isNull);
      },
    );
  }
}
