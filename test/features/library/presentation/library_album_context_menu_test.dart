import "package:cedarflake_ame/features/library/application/library_album_requests.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_album_navigation_tile.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("album pointer, More and keyboard menus share the same actions", (
    tester,
  ) async {
    final edits = <LibraryAlbumEditKind>[];
    var selected = false;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 300,
            child: LibraryAlbumNavigationTile(
              album: const LibraryOrganizationAlbum("one", "Album"),
              selected: false,
              onSelect: () => selected = true,
              onEdit: edits.add,
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text("Album"), buttons: kSecondaryMouseButton);
    await tester.pumpAndSettle();
    expect(selected, isFalse);
    expect(find.text("删除相册"), findsOneWidget);
    await tester.tap(find.text("重命名"));
    await tester.pumpAndSettle();
    expect(edits, [LibraryAlbumEditKind.rename]);
    await tester.tap(
      find.byKey(const ValueKey("library-sidebar-album-menu-one")),
    );
    await tester.pumpAndSettle();
    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();
    tester.widget<ListTile>(find.byType(ListTile)).focusNode!.requestFocus();
    await tester.pump();
    await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.f10);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
    await tester.pumpAndSettle();
    expect(find.text("重命名"), findsOneWidget);
    await tester.tap(find.text("删除相册"));
    await tester.pumpAndSettle();
    expect(edits.last, LibraryAlbumEditKind.delete);
  });

  testWidgets("an album menu cannot edit a replacement row", (tester) async {
    var album = const LibraryOrganizationAlbum("one", "Album");
    late StateSetter rebuild;
    var edits = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return LibraryAlbumNavigationTile(
                album: album,
                selected: false,
                onSelect: () {},
                onEdit: (_) => edits++,
              );
            },
          ),
        ),
      ),
    );
    await tester.tap(find.text("Album"), buttons: kSecondaryMouseButton);
    await tester.pumpAndSettle();
    rebuild(() => album = const LibraryOrganizationAlbum("two", "Another"));
    await tester.pump();
    await tester.tap(find.text("删除相册"));
    await tester.pumpAndSettle();
    expect(edits, 0);
  });
}
