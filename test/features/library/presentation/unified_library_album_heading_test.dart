import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_album_pages.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_header.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";
import "support/library_screen_fixture.dart";

void main() {
  testWidgets("album headings retain the selected name while metadata loads", (
    tester,
  ) async {
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
    final service = _DelayedAlbums()
      ..albumItems = [
        const LibraryOrganizationAlbum("a", "Landscape"),
        const LibraryOrganizationAlbum("b", "Portrait"),
      ];
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
    await tester.tap(find.byKey(const Key("workbench-albums")));
    await tester.pumpAndSettle();

    String heading() => tester
        .widget<LibraryGalleryHeader>(find.byType(LibraryGalleryHeader))
        .galleryTitle;

    await tester.tap(find.byKey(const Key("library-sidebar-album-a")));
    await tester.pumpAndSettle();
    expect(service.reads["a"]!.isCompleted, isFalse);
    expect(heading(), "Landscape");

    await tester.tap(find.byKey(const Key("library-sidebar-album-b")));
    await tester.pumpAndSettle();
    expect(heading(), "Portrait");
    service.reads["a"]!.complete(
      const LibraryOrganizationAlbum("a", "Landscape updated"),
    );
    await tester.pumpAndSettle();
    expect(heading(), "Portrait");
    service.reads["b"]!.complete(
      const LibraryOrganizationAlbum("b", "Portrait updated"),
    );
    await tester.pumpAndSettle();
    expect(heading(), "Portrait updated");

    await tester.tap(find.byKey(const Key("workbench-gallery")));
    await tester.pumpAndSettle();
    service.reads["b"] = Completer<LibraryOrganizationAlbum?>();
    container.invalidate(libraryAlbumProvider("b"));
    await tester.tap(find.byKey(const Key("workbench-albums")));
    await tester.pumpAndSettle();
    expect(heading(), "Portrait updated");
    service.reads["b"]!.complete(null);
    await tester.pumpAndSettle();
    expect(heading(), "相册");
    expect(tester.takeException(), isNull);
  });
}

class _DelayedAlbums extends OrganizationServiceFixture {
  final reads = <String, Completer<LibraryOrganizationAlbum?>>{};

  @override
  Future<LibraryOrganizationAlbum?> findAlbum(String id) =>
      reads.putIfAbsent(id, Completer<LibraryOrganizationAlbum?>.new).future;
}
