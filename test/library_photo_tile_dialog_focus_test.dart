import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/application/library_gallery_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/duplicate_review_fixtures.dart";

void main() {
  for (final keyboard in [false, true]) {
    testWidgets(
      "dialog restores only existing card focus (keyboard=$keyboard)",
      (tester) async {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              libraryControllerProvider.overrideWith(
                () => ReviewFixtureLibrary(
                  duplicateMode: LibraryGalleryDuplicateMode.merged,
                ),
              ),
              libraryGalleryDuplicateAnnotationsProvider.overrideWith(
                (_) async => {"one": reviewGroup},
              ),
              libraryExactDuplicatesProvider.overrideWithValue(_Duplicates()),
            ],
            child: MaterialApp(
              home: Scaffold(
                body: Align(
                  alignment: Alignment.topLeft,
                  child: LibraryPhotoTile(
                    asset: reviewAsset("one"),
                    width: 240,
                    height: 160,
                    isSelecting: false,
                    isSelected: false,
                    onOpen: (_) {},
                    onToggleSelection: (_) {},
                    onViewInformation: (_) {},
                    onCopyPath: (_) {},
                    onRevealFile: (_) {},
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        final badge = find.byKey(const ValueKey("duplicate-badge-one"));
        final select = find.byKey(const ValueKey("select-one"));
        final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
        await mouse.addPointer(location: const Offset(790, 590));
        addTearDown(mouse.removePointer);
        expect(select, findsNothing);
        if (keyboard) {
          tester.widget<IconButton>(badge).focusNode!.requestFocus();
          await tester.pump();
          await tester.sendKeyEvent(LogicalKeyboardKey.enter);
        } else {
          await mouse.moveTo(tester.getCenter(badge));
          await tester.pump();
          expect(select, findsOneWidget);
          await mouse.down(tester.getCenter(badge));
          await mouse.up();
        }
        await tester.pumpAndSettle();
        expect(find.byType(Dialog), findsOneWidget);
        await mouse.moveTo(const Offset(790, 590));
        if (keyboard) {
          await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        } else {
          await tester.tap(find.byKey(const Key("duplicate-locations-close")));
        }
        await tester.pumpAndSettle();
        expect(find.byType(Dialog), findsNothing);
        expect(tester.widget<IconButton>(badge).focusNode!.hasFocus, keyboard);
        expect(select, keyboard ? findsOneWidget : findsNothing);
        expect(tester.takeException(), isNull);
      },
    );
  }
}

class _Duplicates extends Fake implements LibraryExactDuplicates {
  @override
  Future<LibraryDuplicateLocationPage> loadLocations(
    LibraryDuplicateGroup group,
    String? after,
  ) async => const LibraryDuplicateLocationPage(locations: []);
}
