import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_classification_retry.dart";
import "package:cedarflake_ame/features/library/application/library_classification_retry_actions.dart";
import "package:cedarflake_ame/features/library/application/library_classification_settings.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_operation_notice.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_classification.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_organization_feedback.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_classification_retry_fixture.dart";
import "../support/library_classification_review_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  Future<ProviderContainer> mount(
    WidgetTester tester,
    ClassificationRetryFixture retry,
    ClassificationReviewFixture review,
    LibraryClassificationMode mode,
  ) async {
    final container = ProviderContainer(
      overrides: [
        libraryClassificationRetryProvider.overrideWithValue(retry),
        libraryClassificationReviewServiceProvider.overrideWithValue(review),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
        libraryClassificationSettingsProvider.overrideWith(
          (ref) async => classificationRetrySettings(mode: mode),
        ),
      ],
    );
    addTearDown(container.dispose);
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          home: Scaffold(
            body: Stack(
              fit: StackFit.expand,
              children: [
                Align(
                  alignment: Alignment.topRight,
                  child: LibraryViewerClassification(
                    target: organizationTarget("one"),
                  ),
                ),
                Consumer(
                  builder: (context, ref, _) {
                    final notice = ref.watch(
                      libraryViewerOperationNoticeProvider,
                    );
                    return notice == null
                        ? const SizedBox.shrink()
                        : LibraryOrganizationFeedback(notification: notice);
                  },
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    return container;
  }

  ClassificationReviewFixture failed() =>
      ClassificationReviewFixture()
        ..items = const [
          LibraryClassificationItem(
            assetId: "asset-one",
            review: LibraryClassificationReview.unclassified,
            reviewReason: LibraryClassificationReviewReason.analysisFailed,
          ),
        ];

  testWidgets(
    "failed viewer image retries directly and uses the shared top notice",
    (tester) async {
      final retry = ClassificationRetryFixture();
      final review = failed();
      final container = await mount(
        tester,
        retry,
        review,
        LibraryClassificationMode.automatic,
      );
      expect(find.byTooltip("重新分析"), findsOneWidget);
      await tester.tap(find.byTooltip("重新分析"));
      await tester.pumpAndSettle();
      expect(retry.targets, [organizationTarget("one")]);
      expect(retry.revisions, [BigInt.from(7)]);
      expect(review.reads.first, [organizationTarget("one")]);
      expect(find.text("已安排重新分析"), findsOneWidget);
      expect(
        container.read(libraryClassificationRetryActionsProvider).busy,
        isFalse,
      );
      await tester.pump(const Duration(seconds: 6));
    },
  );

  for (final mode in [
    LibraryClassificationMode.disabled,
    LibraryClassificationMode.stopped,
  ]) {
    testWidgets("${mode.name} analysis offers no image retry", (tester) async {
      await mount(tester, ClassificationRetryFixture(), failed(), mode);
      expect(find.byTooltip("重新分析"), findsNothing);
    });
  }

  for (final status in LibraryClassificationReview.values) {
    testWidgets("${status.name} has no manual classification controls", (
      tester,
    ) async {
      final service = ClassificationReviewFixture()
        ..items = [
          LibraryClassificationItem(
            assetId: "asset-one",
            categoryId: "drawing",
            review: status,
          ),
        ];
      await mount(
        tester,
        ClassificationRetryFixture(),
        service,
        LibraryClassificationMode.automatic,
      );
      expect(find.text("插画"), findsOneWidget);
      expect(find.byType(PopupMenuButton<String>), findsNothing);
      expect(find.byTooltip("重新分析"), findsNothing);
      for (final label in ["确认分类", "更改分类", "稍后确认", "待确认", "已确认"]) {
        expect(find.text(label), findsNothing);
      }
    });
  }

  testWidgets("an ordinary suggestion offers no failed-analysis retry", (
    tester,
  ) async {
    await mount(
      tester,
      ClassificationRetryFixture(),
      ClassificationReviewFixture(),
      LibraryClassificationMode.automatic,
    );
    expect(find.byTooltip("重新分析"), findsNothing);
  });
}
