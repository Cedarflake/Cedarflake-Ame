import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_ui/material_ui.dart";

import "../features/library/presentation/support/library_screen_fixture.dart";
import "../support/application_test_bindings.dart";
import "../support/desktop_language_workflow.dart";

void main() {
  testWidgets(
    "switches a populated workbench and acts on its retained notification",
    (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(960, 700);
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initial = populatedScreenState(totalItems: 1);
      final catalog = RecordingQueryCatalog(
        LibrarySnapshot(
          catalogPath: initial.catalogPath!,
          revision: initial.catalogRevision!,
          queryId: initial.queryId,
          roots: initial.roots,
          assets: initial.assets,
        ),
      );
      await tester.pumpWidget(
        ProviderScope(
          overrides: applicationTestOverrides([
            initialLibraryStateProvider.overrideWithValue(initial),
            libraryCatalogProvider.overrideWithValue(catalog),
          ]),
          child: const AmeApp(),
        ),
      );
      await tester.pumpAndSettle();
      final reads = catalog.loadQueries.length;
      await verifyDesktopLanguageWorkflow(tester);
      expect(catalog.loadQueries.length, reads);
    },
  );
}
