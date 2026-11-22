import "package:cedarflake_ame/features/library/presentation/widgets/library_sidebar_section_toggle.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/localized_test_app.dart";

void main() {
  for (final (locale, section, collapse, expand) in [
    (
      const Locale.fromSubtags(languageCode: "zh", scriptCode: "Hans"),
      "相册",
      "收起相册",
      "展开相册",
    ),
    (const Locale("en"), "Albums", "Collapse Albums", "Expand Albums"),
    (const Locale("ja"), "アルバム", "アルバムを折りたたむ", "アルバムを展開"),
  ]) {
    testWidgets("$locale sidebar actions use complete localized labels", (
      tester,
    ) async {
      var expanded = true;
      await tester.pumpWidget(
        LocalizedTestApp(
          locale: locale,
          home: Scaffold(
            body: StatefulBuilder(
              builder: (context, setState) => LibrarySidebarSectionToggle(
                label: section,
                expanded: expanded,
                onPressed: () => setState(() => expanded = !expanded),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      for (final action in [collapse, expand, collapse]) {
        final button = find.byTooltip(action);
        expect(button, findsOneWidget);
        expect(tester.getSemantics(button).getSemanticsData().tooltip, action);
        await tester.tap(button);
        await tester.pumpAndSettle();
      }
    });
  }
}
