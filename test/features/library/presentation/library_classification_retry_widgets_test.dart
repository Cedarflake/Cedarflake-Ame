import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_classification_retry.dart";
import "package:cedarflake_ame/features/library/application/library_classification_retry_actions.dart";
import "package:cedarflake_ame/features/library/application/library_classification_settings.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_operation_notice.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_classification_controls.dart";
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
                  child: LibraryClassificationControls(
                    enabled: true,
                    viewer: true,
                    scope: LibraryClassificationScope(
                      query: const LibraryGalleryQuery(),
                      catalogRevision: BigInt.one,
                      allMatching: false,
                      assetIds: const [],
                      targets: [organizationTarget("one")],
                    ),
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
    await tester.tap(find.byKey(const Key("library-change-classification")));
    await tester.pumpAndSettle();
    return container;
  }

  ClassificationReviewFixture failed() =>
      ClassificationReviewFixture()
        ..summary = const LibraryClassificationSummary(
          groups: 1,
          confirmable: 0,
          confirmed: 0,
          singleItem: LibraryClassificationItem(
            assetId: "asset-one",
            review: LibraryClassificationReview.unclassified,
            canConfirm: false,
            reviewReason: LibraryClassificationReviewReason.analysisFailed,
          ),
        );

  testWidgets(
    "failed viewer image retries through its category menu and shared top notice",
    (tester) async {
      final retry = ClassificationRetryFixture();
      final review = failed();
      final container = await mount(
        tester,
        retry,
        review,
        LibraryClassificationMode.automatic,
      );
      expect(find.text("重新分析"), findsOneWidget);
      await tester.tap(find.text("重新分析"));
      await tester.pumpAndSettle();
      expect(retry.targets, [organizationTarget("one")]);
      expect(retry.revisions, [BigInt.from(7)]);
      expect(review.drafts.expand((draft) => draft.commands), isEmpty);
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
      expect(find.text("重新分析"), findsNothing);
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
    expect(find.text("重新分析"), findsNothing);
  });
}
