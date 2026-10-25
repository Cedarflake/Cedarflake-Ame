import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_states.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

void main() {
  testWidgets(
    "entering, switching and leaving review retain published empty content",
    (tester) async {
      var imports = 0;
      final state = LibraryState(catalogRevision: BigInt.one);
      Future<void> show(LibraryState value) => tester.pumpWidget(
        MaterialApp(
          home: EmptyLibraryGallery(state: value, onImport: () => imports++),
        ),
      );
      await show(state);
      final title = find.text("建立你的图片图库");
      final before = tester.getRect(title);
      for (final queue in [
        LibraryOrganizationQueue.pending,
        LibraryOrganizationQueue.deferred,
        null,
      ]) {
        await show(
          state.copyWith(
            queryActivity: LibraryQueryLoading(
              LibraryGalleryQuery(organizationQueue: queue),
            ),
            isLoadingTimeline: true,
          ),
        );
        await tester.pump(const Duration(milliseconds: 100));
        expect(tester.getRect(title), before);
        expect(find.byType(CircularProgressIndicator), findsNothing);
        expect(find.byIcon(Symbols.hourglass_top_rounded), findsNothing);
        final import = find.widgetWithText(FilledButton, "导入");
        expect(tester.widget<FilledButton>(import).onPressed, isNotNull);
        await tester.tapAt(tester.getCenter(import));
        tester.widget<FilledButton>(import).onPressed!();
        expect(imports, 0);
        await show(
          state.copyWith(query: LibraryGalleryQuery(organizationQueue: queue)),
        );
        expect(tester.getRect(title), before);
      }
      await tester.tap(find.widgetWithText(FilledButton, "导入"));
      expect(imports, 1);
    },
  );

  testWidgets("initial catalog loading still has progress feedback", (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: EmptyLibraryGallery(
          state: const LibraryState(
            queryActivity: LibraryQueryLoading(LibraryGalleryQuery()),
          ),
          onImport: () {},
        ),
      ),
    );
    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    expect(find.byKey(const Key("library-empty-state")), findsNothing);
  });
}
