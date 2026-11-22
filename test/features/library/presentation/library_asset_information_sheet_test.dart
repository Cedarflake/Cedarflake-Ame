import "package:cedarflake_ame/app/presentation/ame_localizations.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_asset_information_sheet.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_ui/material_ui.dart";

import "../../../support/duplicate_review_fixtures.dart";
import "../../../support/localized_test_app.dart";

void main() {
  for (final locale in ameSupportedLocales) {
    testWidgets("information columns stay separated in $locale", (
      tester,
    ) async {
      tester.view.physicalSize = const Size(800, 560);
      tester.view.devicePixelRatio = 1;
      tester.platformDispatcher.textScaleFactorTestValue = 1.5;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
      await tester.pumpWidget(
        LocalizedTestApp(
          locale: locale,
          home: Builder(
            builder: (context) => FilledButton(
              onPressed: () =>
                  showLibraryAssetInformation(context, reviewAsset("one")),
              child: const Text("Open information"),
            ),
          ),
        ),
      );
      await tester.tap(find.text("Open information"));
      await tester.pumpAndSettle();
      final values = find.byType(SelectableText);
      final valueLeft = tester.getTopLeft(values.first).dx;
      final strings = tester.element(values.first).l10n;
      for (final label in [
        strings.file,
        strings.path,
        strings.fileType,
        strings.imageDimensions,
        strings.size,
        strings.cameraMake,
        strings.cameraModel,
        strings.timeTaken,
        strings.timeCreated,
        strings.timeModified,
      ]) {
        expect(
          valueLeft - tester.getTopRight(find.text(label)).dx,
          greaterThanOrEqualTo(16),
        );
      }
      for (final value in values.evaluate()) {
        expect(tester.getTopLeft(find.byWidget(value.widget)).dx, valueLeft);
      }
      expect(find.text(r"C:\fixture\one.png"), findsOneWidget);
      expect(tester.takeException(), isNull);
    });
  }
}
