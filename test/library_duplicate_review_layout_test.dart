import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_duplicate_review_grid.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/application_test_bindings.dart";
import "support/duplicate_review_fixtures.dart";
import "support/localized_test_app.dart";

void main() {
  testWidgets(
    "grid preview demand covers visible columns and retires when hidden",
    (tester) async {
      await tester.binding.setSurfaceSize(const Size(1280, 900));
      addTearDown(() => tester.binding.setSurfaceSize(null));
      final library = ReviewFixtureLibrary();
      final active = ValueNotifier(true);
      addTearDown(active.dispose);
      await tester.pumpWidget(
        ProviderScope(
          overrides: applicationTestOverrides([
            libraryControllerProvider.overrideWith(() => library),
          ]),
          child: LocalizedTestApp(
            home: Scaffold(
              body: Center(
                child: SizedBox(
                  width: 1200,
                  height: 280,
                  child: ValueListenableBuilder(
                    valueListenable: active,
                    builder: (_, enabled, _) => TickerMode(
                      enabled: enabled,
                      child: LibraryDuplicateReviewGrid(
                        itemCount: 64,
                        assetAt: (index) => reviewAsset("item-$index"),
                        itemBuilder: (_, index) => Text("Image $index"),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(
        library.demands.values.single,
        List.generate(9, (index) => "item-$index"),
      );
      final grid = tester.widget<GridView>(find.byType(GridView));
      final layout =
          grid.gridDelegate as SliverGridDelegateWithFixedCrossAxisCount;
      grid.controller!.jumpTo(
        4 * (layout.mainAxisExtent! + layout.mainAxisSpacing),
      );
      await tester.pumpAndSettle();
      expect(
        library.demands.values.single,
        List.generate(9, (index) => "item-${index + 12}"),
      );
      active.value = false;
      await tester.pumpAndSettle();
      expect(library.demands, isEmpty);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );
}
