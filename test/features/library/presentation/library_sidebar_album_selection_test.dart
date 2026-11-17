import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_gallery_collection.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_album_navigation.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_folder_disclosure.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

import "../../../support/application_test_bindings.dart";

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
      Widget view(bool active, {double width = 220}) => ProviderScope(
        overrides: applicationTestOverrides([
          libraryOrganizationProvider.overrideWithValue(service),
          libraryControllerProvider.overrideWith(
            OrganizationLibraryFixture.new,
          ),
        ]),
        child: MaterialApp(
          home: Scaffold(
            body: SizedBox(
              width: width,
              child: LibraryAlbumNavigation(
                isActive: active,
                collection: const LibraryGalleryCollection.library(),
                enabled: true,
                onSelect: (album) => selected = album.id,
                header: const Text("收藏夹"),
              ),
            ),
          ),
        ),
      );
      await tester.pumpWidget(view(true));
      await tester.pumpAndSettle();
      final firstAlbum = find.text("First");
      final albumBounds = tester.getRect(firstAlbum);
      final create = find.byKey(const Key("library-sidebar-create-album"));
      final countStyle = tester.widget<Text>(find.text("(2)")).style;
      expect(create, findsOneWidget);
      await tester.tap(
        find.byKey(const ValueKey("library-sidebar-select-album-one")),
      );
      await tester.pumpAndSettle();
      expect(selected, isEmpty);
      expect(find.text("已选"), findsOneWidget);
      expect(tester.getRect(firstAlbum), albumBounds);
      expect(create, findsNothing);
      await tester.pumpWidget(view(true, width: 260));
      await tester.pumpAndSettle();
      expect(find.text("已选择"), findsOneWidget);
      expect(find.text("(1)"), findsOneWidget);
      await tester.pumpWidget(view(true));
      await tester.pumpAndSettle();
      expect(find.text("已选"), findsOneWidget);
      await tester.pumpWidget(view(false));
      await tester.pumpWidget(view(true));
      await tester.pumpAndSettle();
      expect(find.text("(1)"), findsNothing);
      expect(find.text("已选择"), findsNothing);
      expect(create, findsOneWidget);
      await tester.tap(
        find.byKey(const ValueKey("library-sidebar-select-album-one")),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key("library-sidebar-select-all")));
      await tester.pumpAndSettle();
      expect(find.text("(2)"), findsOneWidget);
      expect(tester.widget<Text>(find.text("(2)")).style, countStyle);
      expect(tester.getRect(firstAlbum), albumBounds);
      expect(
        tester.getCenter(find.text("(2)")).dy,
        tester
            .getCenter(find.byKey(const Key("library-sidebar-select-all")))
            .dy,
      );
      final collapse = find.byKey(const Key("library-sidebar-album-expand"));
      await tester.tap(collapse);
      await tester.pumpAndSettle();
      expect(find.text("First"), findsNothing);
      expect(find.text("Second"), findsNothing);
      expect(find.text("(2)"), findsOneWidget);
      expect(service.albumWrites, isEmpty);
      expect(selected, isEmpty);
      await tester.tap(collapse);
      await tester.pumpAndSettle();
      expect(find.text("First"), findsOneWidget);
      expect(find.text("Second"), findsOneWidget);
      expect(find.text("(2)"), findsOneWidget);
      final remove = find.byKey(const Key("library-sidebar-remove-selected"));
      await tester.tap(remove);
      await tester.pumpAndSettle();
      expect(find.text("删除 2 个相册？"), findsOneWidget);
      expect(service.albumWrites, isEmpty);
      await tester.tap(find.text("取消"));
      await tester.pumpAndSettle();
      expect(find.text("(2)"), findsOneWidget);
      await tester.tap(remove);
      await tester.pumpAndSettle();
      await tester.tap(find.text("删除相册"));
      await tester.pumpAndSettle();
      expect(service.albumWrites.map((w) => w.albumId), ["one", "two"]);
      expect(find.text("(2)"), findsNothing);
      expect(create, findsOneWidget);
      expect(find.text("相册"), findsOneWidget);
      final empty = find.byKey(const Key("library-sidebar-album-empty"));
      expect(empty, findsOneWidget);
      final disclosure = find.ancestor(
        of: empty,
        matching: find.byType(LibraryFolderDisclosure),
      );
      final expandedHeight = tester.getSize(disclosure).height;
      await tester.tap(collapse);
      await tester.pump();
      expect(tester.getSize(disclosure).height, expandedHeight);
      await tester.pump(const Duration(milliseconds: 60));
      expect(
        tester.getSize(disclosure).height,
        inExclusiveRange(0, expandedHeight),
      );
      await tester.pumpAndSettle();
      expect(empty, findsNothing);
      await tester.tap(collapse);
      await tester.pump();
      expect(tester.getSize(disclosure).height, 0);
      await tester.pump(const Duration(milliseconds: 60));
      expect(
        tester.getSize(disclosure).height,
        inExclusiveRange(0, expandedHeight),
      );
      await tester.pumpAndSettle();
      expect(tester.getSize(disclosure).height, expandedHeight);
      expect(tester.takeException(), isNull);
    },
  );
}
