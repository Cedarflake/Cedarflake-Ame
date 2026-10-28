import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_gallery_collection.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_album_navigation.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

void main() {
  testWidgets(
    "album multi-selection confirms once, cancellation preserves it and deletion clears it",
    (tester) async {
      final service = OrganizationServiceFixture()
        ..albumItems = const [
          LibraryOrganizationAlbum("one", "First"),
          LibraryOrganizationAlbum("two", "Second"),
        ];
      var selected = "";
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            libraryOrganizationProvider.overrideWithValue(service),
            libraryControllerProvider.overrideWith(
              OrganizationLibraryFixture.new,
            ),
          ],
          child: MaterialApp(
            home: Scaffold(
              body: SizedBox(
                width: 300,
                child: LibraryAlbumNavigation(
                  collection: const LibraryGalleryCollection.library(),
                  enabled: true,
                  onSelect: (album) => selected = album.id,
                  header: const Text("收藏夹"),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const ValueKey("library-sidebar-select-album-one")),
      );
      await tester.pumpAndSettle();
      expect(selected, isEmpty);
      await tester.tap(find.byKey(const Key("library-sidebar-select-all")));
      await tester.pumpAndSettle();
      expect(find.text("已选择 2"), findsOneWidget);
      final remove = find.byKey(const Key("library-sidebar-remove-selected"));
      final create = find.byKey(const Key("library-sidebar-create-album"));
      expect(
        tester.getCenter(remove).dx,
        lessThan(tester.getCenter(create).dx),
      );
      await tester.tap(remove);
      await tester.pumpAndSettle();
      expect(find.text("删除 2 个相册？"), findsOneWidget);
      expect(service.albumWrites, isEmpty);
      await tester.tap(find.text("取消"));
      await tester.pumpAndSettle();
      expect(find.text("已选择 2"), findsOneWidget);
      await tester.tap(remove);
      await tester.pumpAndSettle();
      await tester.tap(find.text("删除相册"));
      await tester.pumpAndSettle();
      expect(service.albumWrites.map((w) => w.albumId), ["one", "two"]);
      expect(find.text("已选择 2"), findsNothing);
      expect(find.text("相册"), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
}
