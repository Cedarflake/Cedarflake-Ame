import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_states.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_viewport.dart";
import "package:cedarflake_ame/app/presentation/ame_loading_indicator.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../support/localized_test_app.dart";

void main() {
  testWidgets(
    "entering, switching and leaving review retain published empty content",
    (tester) async {
      var imports = 0;
      final state = LibraryState(catalogRevision: BigInt.one);
      Future<void> show(LibraryState value) => tester.pumpWidget(
        LocalizedTestApp(
          home: LibraryGalleryViewport(
            header: const SizedBox(height: 80),
            isEmpty: true,
            isLoading: value.isRefreshingQuery,
            hasPublishedResults: value.catalogRevision != null,
            body: EmptyLibraryGallery(state: value, onImport: () => imports++),
          ),
        ),
      );
      await show(state);
      final emptyState = find.byKey(const Key("library-empty-state"));
      final import = find.descendant(
        of: emptyState,
        matching: find.byType(FilledButton),
      );
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
        expect(emptyState, findsOneWidget);
        expect(find.byType(CircularProgressIndicator), findsNothing);
        expect(find.byIcon(Symbols.hourglass_top_rounded), findsNothing);
        expect(tester.widget<FilledButton>(import).onPressed, isNotNull);
        await tester.tapAt(tester.getCenter(import));
        tester.widget<FilledButton>(import).onPressed!();
        expect(imports, 0);
        await show(
          state.copyWith(query: LibraryGalleryQuery(organizationQueue: queue)),
        );
        expect(emptyState, findsOneWidget);
      }
      await tester.tap(import);
      expect(imports, 1);
    },
  );

  testWidgets("initial catalog loading still has progress feedback", (
    tester,
  ) async {
    await tester.pumpWidget(
      LocalizedTestApp(
        home: EmptyLibraryGallery(
          state: const LibraryState(
            queryActivity: LibraryQueryLoading(LibraryGalleryQuery()),
          ),
          onImport: () {},
        ),
      ),
    );
    expect(find.byType(AmeLoadingIndicator), findsOneWidget);
    expect(find.byKey(const Key("library-empty-state")), findsNothing);
  });
}
