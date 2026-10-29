import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_review.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";
import "../support/library_review_fixture.dart";
import "support/library_screen_fixture.dart";

void main() {
  testWidgets(
    "first Albums visit during mode entry keeps collection actions usable",
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
          queryId: state.queryId,
          roots: state.roots,
          assets: state.assets,
        ),
      );
      final review = _PendingEntry();
      final organization = OrganizationServiceFixture()
        ..albumItems = [const LibraryOrganizationAlbum("a", "Travel")];
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryOrganizationProvider.overrideWithValue(organization),
          libraryReviewProvider.overrideWithValue(review),
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
      await tester.tap(find.text("整理模式"));
      await tester.pump(const Duration(milliseconds: 300));
      await tester.tap(find.byKey(const Key("workbench-albums")));
      await tester.pump();
      review.boundary.complete(BigInt.one);
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("library-review-toolbar")), findsNothing);
      await tester.tap(find.byKey(const Key("library-sidebar-album-a")));
      await tester.pumpAndSettle();
      expect(
        catalog.loadQueries.last.collection,
        const LibraryGalleryCollection.album("a"),
      );
      await tester.tap(find.byKey(const Key("library-sidebar-favorites")));
      await tester.pumpAndSettle();
      expect(
        catalog.loadQueries.last.collection,
        const LibraryGalleryCollection.favorites(),
      );
      await tester.tap(find.byKey(const Key("workbench-gallery")));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key("library-review-toolbar")), findsOneWidget);
      expect(
        catalog.loadQueries.last.organizationQueue,
        LibraryOrganizationQueue.pending,
      );
      expect(tester.takeException(), isNull);
    },
  );
}

class _PendingEntry extends ReviewServiceFixture {
  final boundary = Completer<BigInt>();

  @override
  Future<BigInt> admissionBoundary() => boundary.future;
}
