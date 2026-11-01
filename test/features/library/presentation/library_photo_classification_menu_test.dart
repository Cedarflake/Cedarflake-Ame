import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_classification_retry.dart";
import "package:cedarflake_ame/features/library/application/library_classification_retry_actions.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_classification_settings.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_context_menu.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_classification_review_fixture.dart";
import "../support/library_classification_retry_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  late ClassificationReviewFixture service;
  late OrganizationServiceFixture organization;
  late _Library library;
  late ProviderContainer container;
  late LibraryPhotoContextMenu menu;
  late ClassificationRetryFixture retry;
  final target = organizationTarget("one");

  setUp(() {
    retry = ClassificationRetryFixture();
    service = ClassificationReviewFixture();
    organization = OrganizationServiceFixture();
    library = _Library();
    container = ProviderContainer(
      overrides: [
        libraryClassificationRetryProvider.overrideWithValue(retry),
        libraryClassificationSettingsProvider.overrideWith((ref) async {
          ref.keepAlive();
          return classificationRetrySettings();
        }),
        libraryClassificationReviewServiceProvider.overrideWithValue(service),
        libraryOrganizationProvider.overrideWithValue(organization),
        libraryControllerProvider.overrideWith(() => library),
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
    menu = LibraryPhotoContextMenu();
  });

  tearDown(() {
    menu.dispose();
    container.dispose();
  });

  Future<void> open(WidgetTester tester) async {
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          home: Scaffold(
            body: Consumer(
              builder: (context, ref, _) => TextButton(
                onPressed: () => unawaited(
                  menu.open(
                    context: context,
                    ref: ref,
                    position: const RelativeRect.fromLTRB(10, 40, 500, 0),
                    target: target,
                    isCurrent: () => true,
                    onOpen: () {},
                    onInformation: () {},
                    onCopyPath: () {},
                    onReveal: () {},
                  ),
                ),
                child: const Text("Menu"),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text("Menu"));
    await tester.pumpAndSettle();
  }

  for (final status in LibraryClassificationReview.values) {
    testWidgets(
      "${status.name} keeps ordinary actions without manual classification",
      (tester) async {
        service.items = [
          LibraryClassificationItem(
            assetId: "asset-one",
            categoryId: "drawing",
            review: status,
          ),
        ];
        await open(tester);
        expect(find.text("加入收藏夹"), findsOneWidget);
        expect(find.text("加入相册"), findsOneWidget);
        for (final label in ["确认分类", "更改分类", "稍后确认", "重新分析"]) {
          expect(find.text(label), findsNothing);
        }
        expect(organization.writes, isEmpty);
        expect(organization.memberships, isEmpty);
        expect(
          container.read(ameNotificationControllerProvider).history,
          isEmpty,
        );
        await tester.tap(find.text("复制路径"));
        await tester.pumpAndSettle();
      },
    );
  }

  void failImage() {
    service.items = const [
      LibraryClassificationItem(
        assetId: "asset-one",
        review: LibraryClassificationReview.unclassified,
        reviewReason: LibraryClassificationReviewReason.analysisFailed,
      ),
    ];
  }

  testWidgets("failed-image context retry preserves the exact target", (
    tester,
  ) async {
    failImage();
    await container.read(libraryClassificationSettingsProvider.future);
    await open(tester);
    await tester.tap(find.text("重新分析"));
    await tester.pumpAndSettle();
    expect(retry.targets, [target]);
    expect(retry.revisions, [BigInt.from(7)]);
    expect(service.reads.single, [target]);
    expect(
      container
          .read(libraryClassificationRetryActionsProvider)
          .notice!
          .actionId,
      isNull,
    );
  });

  testWidgets("a replaced query retires a late classification inspection", (
    tester,
  ) async {
    service.pendingRead = Completer<List<LibraryClassificationItem>>();
    await open(tester);
    library.leaveClassification();
    failImage();
    service.pendingRead!.complete(service.items);
    await tester.pumpAndSettle();
    expect(find.text("重新分析"), findsNothing);
    expect(find.text("复制路径"), findsNothing);
    expect(retry.targets, isEmpty);
  });

  testWidgets("a query switch rejects an already-open retry", (tester) async {
    failImage();
    await container.read(libraryClassificationSettingsProvider.future);
    await open(tester);
    library.leaveClassification();
    await tester.tap(find.text("重新分析"));
    await tester.pumpAndSettle();
    expect(retry.targets, isEmpty);
  });

  testWidgets(
    "inspection failure retains basic actions and reopening retries",
    (tester) async {
      service.pendingRead = Completer<List<LibraryClassificationItem>>();
      await open(tester);
      service.pendingRead!.completeError(
        const LibraryClassificationFailure("busy", "busy"),
      );
      await tester.pumpAndSettle();
      expect(find.text("复制路径"), findsOneWidget);
      expect(
        container.read(ameNotificationControllerProvider).history.single.title,
        "暂时无法读取分类",
      );
      await tester.tap(find.text("复制路径"));
      await tester.pumpAndSettle();
      service.pendingRead = null;
      failImage();
      await container.read(libraryClassificationSettingsProvider.future);
      await open(tester);
      expect(find.text("重新分析"), findsOneWidget);
      expect(
        container
            .read(ameNotificationControllerProvider)
            .history
            .single
            .isActive,
        isFalse,
      );
      await tester.tap(find.text("复制路径"));
      await tester.pumpAndSettle();
    },
  );
}

class _Library extends OrganizationLibraryFixture {
  @override
  LibraryState build() => super.build().copyWith(
    query: const LibraryGalleryQuery(
      collection: LibraryGalleryCollection.smartAlbum(categoryId: "drawing"),
    ),
  );

  void leaveClassification() =>
      state = state.copyWith(query: const LibraryGalleryQuery());
}
