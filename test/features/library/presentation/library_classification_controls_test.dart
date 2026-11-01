import "package:cedarflake_ame/features/library/application/library_classification_actions.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_classification_settings.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_classification_controls.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_classification_review_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  Future<ProviderContainer> mount(
    WidgetTester tester,
    ClassificationReviewFixture service, {
    bool viewer = false,
  }) async {
    final container = ProviderContainer(
      overrides: [
        libraryClassificationReviewServiceProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
        libraryClassificationCategoriesProvider.overrideWith(
          (ref) => const AsyncData([
            LibraryClassificationCategory(
              id: "drawing",
              label: "插画",
              descriptions: ["drawing"],
            ),
            LibraryClassificationCategory(
              id: "photo",
              label: "实拍",
              descriptions: ["photo"],
            ),
          ]),
        ),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          home: Scaffold(
            body: Align(
              alignment: Alignment.topRight,
              child: LibraryClassificationControls(
                scope: classificationScope(),
                enabled: true,
                viewer: viewer,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return container;
  }

  testWidgets(
    "mixed selection chooses a category from the anchored menu without a dialog",
    (tester) async {
      final service = ClassificationReviewFixture();
      final container = await mount(tester, service);
      await tester.tap(find.byKey(const Key("library-change-classification")));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(
        tester
            .widget<Semantics>(find.byKey(const ValueKey("menu-choice-插画")))
            .properties
            .checked,
        isFalse,
      );
      expect(
        tester
            .widget<Semantics>(find.byKey(const ValueKey("menu-choice-实拍")))
            .properties
            .checked,
        isFalse,
      );
      await tester.tap(find.text("实拍"));
      await tester.pumpAndSettle();
      expect(
        service.drafts.first.commands.single,
        isA<LibraryCorrectClassification>().having(
          (command) => command.categoryId,
          "category",
          "photo",
        ),
      );
      expect(
        container.read(libraryClassificationActionsProvider).notice?.title,
        "已更改分类",
      );
    },
  );

  testWidgets(
    "confirmed viewer shows its category and disables redundant confirmation",
    (tester) async {
      final service = ClassificationReviewFixture()
        ..summary = const LibraryClassificationSummary(
          groups: 1,
          confirmable: 0,
          confirmed: 1,
          commonCategoryId: "drawing",
        );
      await mount(tester, service, viewer: true);
      expect(find.text("插画"), findsOneWidget);
      expect(
        find.byKey(const Key("library-confirm-classification")),
        findsNothing,
      );
      await tester.tap(find.byKey(const Key("library-change-classification")));
      await tester.pumpAndSettle();
      final confirm = tester.widget<PopupMenuItem<String>>(
        find.ancestor(
          of: find.text("确认分类"),
          matching: find.byType(PopupMenuItem<String>),
        ),
      );
      expect(confirm.enabled, isFalse);
      expect(
        tester
            .widget<Semantics>(find.byKey(const ValueKey("menu-choice-插画")))
            .properties
            .checked,
        isTrue,
      );
    },
  );
}
