import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_classification_settings.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_operation_notice.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/presentation/library_classification_notifications.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_classification.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_organization_feedback.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_classification_review_fixture.dart";
import "../support/library_organization_fixture.dart";

import "../../../support/application_test_bindings.dart";

void main() {
  Future<ProviderContainer> mount(
    WidgetTester tester,
    ClassificationReviewFixture review,
    LibraryClassificationMode mode,
  ) async {
    final container = ProviderContainer(
      overrides: applicationTestOverrides([
        libraryClassificationReviewServiceProvider.overrideWithValue(review),
        libraryControllerProvider.overrideWith(OrganizationLibraryFixture.new),
        libraryClassificationSettingsProvider.overrideWith(
          (ref) async => LibraryClassificationSettings(
            revision: BigInt.one,
            dispatchRevision: BigInt.from(7),
            mode: mode,
            categories: const [
              LibraryClassificationCategory(
                id: "drawing",
                label: "插画",
                descriptions: ["drawing"],
              ),
            ],
            runId: "run",
            installationSelected: true,
          ),
        ),
      ]),
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

  for (final mode in LibraryClassificationMode.values) {
    testWidgets("${mode.name} handles per-image analysis failure silently", (
      tester,
    ) async {
      final review = ClassificationReviewFixture()
        ..items = const [
          LibraryClassificationItem(
            assetId: "asset-one",
            review: LibraryClassificationReview.unclassified,
            reviewReason: LibraryClassificationReviewReason.analysisFailed,
          ),
        ];
      await mount(tester, review, mode);
      expect(find.byType(Tooltip), findsNothing);
      expect(find.byType(Icon), findsNothing);
      expect(find.text("分析失败"), findsNothing);
      expect(find.byType(IconButton), findsNothing);
      expect(review.reads, [
        [organizationTarget("one")],
      ]);
    });
  }

  for (final status in LibraryClassificationReview.values) {
    testWidgets("${status.name} retains a read-only category", (tester) async {
      final service = ClassificationReviewFixture()
        ..items = [
          LibraryClassificationItem(
            assetId: "asset-one",
            categoryId: "drawing",
            review: status,
          ),
        ];
      await mount(tester, service, LibraryClassificationMode.automatic);
      expect(find.text("插画"), findsOneWidget);
      expect(find.byType(IconButton), findsNothing);
      expect(find.byType(PopupMenuButton<String>), findsNothing);
    });
  }

  testWidgets(
    "runtime failures use the shared preview notice and clear on recovery",
    (tester) async {
      final container = await mount(
        tester,
        ClassificationReviewFixture(),
        LibraryClassificationMode.automatic,
      );
      final feedback = LibraryClassificationNotifications(
        container.read(ameNotificationControllerProvider.notifier),
      );
      feedback.accept(StateError("worker unavailable"));
      await tester.pumpAndSettle();
      expect(find.text("智能相册分析已中断：已有分类结果仍可浏览。"), findsOneWidget);
      expect(find.text("重新分析"), findsNothing);
      feedback.accept(null);
      await tester.pumpAndSettle();
      expect(find.textContaining("智能相册分析已中断"), findsNothing);
    },
  );
}
