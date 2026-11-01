import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_classification_actions.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_classification_progress.dart";
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
import "../support/library_classification_progress_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  late ClassificationReviewFixture service;
  late ClassificationProgressFixture progress;
  late OrganizationServiceFixture organization;
  late _Library library;
  late ProviderContainer container;
  late LibraryPhotoContextMenu menu;
  final target = organizationTarget("one");

  setUp(() {
    progress = ClassificationProgressFixture();
    service = ClassificationReviewFixture()
      ..summary = const LibraryClassificationSummary(
        groups: 1,
        confirmable: 1,
        confirmed: 0,
        commonCategoryId: "drawing",
      );
    organization = OrganizationServiceFixture();
    library = _Library();
    container = ProviderContainer(
      overrides: [
        libraryClassificationProgressProvider.overrideWithValue(progress),
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

  testWidgets(
    "confirmation freezes the clicked version and exposes shared undo",
    (tester) async {
      await open(tester);
      expect(find.text("加入收藏夹"), findsOneWidget);
      await tester.tap(find.text("确认分类"));
      await tester.pumpAndSettle();
      final draft = service.drafts.single;
      expect(draft.targets, [target]);
      expect(draft.commands.single, isA<LibraryConfirmClassification>());
      expect(draft.closes, 1);
      expect(progress.targets, [target]);
      expect(draft.reviewPositions, hasLength(1));
      expect(service.scopes.single.allMatching, isFalse);
      expect(organization.writes, isEmpty);
      expect(organization.memberships, isEmpty);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      final actions = container.read(
        libraryClassificationActionsProvider.notifier,
      );
      final notice = container
          .read(libraryClassificationActionsProvider)
          .notice!;
      expect(actions.handleNotification(notice), isTrue);
      await tester.pumpAndSettle();
      expect(service.undos, [draft.operationIds.single]);
    },
  );

  testWidgets(
    "context deferral saves classification progress without organization changes",
    (tester) async {
      service.summary = const LibraryClassificationSummary(
        groups: 1,
        confirmable: 1,
        confirmed: 0,
        commonCategoryId: "drawing",
        singleItem: LibraryClassificationItem(
          assetId: "asset-one",
          review: LibraryClassificationReview.needsReview,
          canConfirm: true,
        ),
      );
      await open(tester);
      await tester.tap(find.text("稍后确认"));
      await tester.pumpAndSettle();
      expect(
        service.drafts.single.commands.single,
        isA<LibraryDeferClassification>(),
      );
      expect(progress.targets, [target]);
      expect(progress.ended, ["review"]);
      expect(organization.writes, isEmpty);
      expect(
        container.read(libraryClassificationActionsProvider).notice?.title,
        "已留待确认",
      );
    },
  );

  testWidgets(
    "confirmed items offer correction with the shared dot selection",
    (tester) async {
      service.summary = const LibraryClassificationSummary(
        groups: 1,
        confirmable: 0,
        confirmed: 1,
        commonCategoryId: "drawing",
      );
      await open(tester);
      expect(find.text("确认分类"), findsNothing);
      await tester.tap(find.text("更改分类"));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      final chosen = tester.widget<Semantics>(
        find.byKey(const ValueKey("menu-choice-插画")),
      );
      expect(chosen.properties.checked, isTrue);
      await tester.tap(find.text("实拍"));
      await tester.pumpAndSettle();
      expect(
        service.drafts.single.commands.single,
        isA<LibraryCorrectClassification>().having(
          (command) => command.categoryId,
          "category",
          "photo",
        ),
      );
      expect(service.drafts.single.closes, 1);
    },
  );

  testWidgets("a replaced query retires a late classification inspection", (
    tester,
  ) async {
    service.pendingOpen = Completer<LibraryClassificationDraft>();
    await open(tester);
    library.leaveClassification();
    final draft = ClassificationDraftFixture(service);
    service.pendingOpen!.complete(draft);
    await tester.pumpAndSettle();
    expect(find.text("确认分类"), findsNothing);
    expect(find.text("复制路径"), findsNothing);
    expect(draft.commands, isEmpty);
    expect(draft.closes, 1);
  });

  testWidgets("a query switch rejects an already-open menu command", (
    tester,
  ) async {
    await open(tester);
    library.leaveClassification();
    await tester.tap(find.text("确认分类"));
    await tester.pumpAndSettle();
    expect(service.drafts.single.commands, isEmpty);
    expect(service.drafts.single.closes, 1);
  });

  testWidgets(
    "inspection failure retains basic actions and reopening retries",
    (tester) async {
      service.pendingOpen = Completer<LibraryClassificationDraft>();
      await open(tester);
      service.pendingOpen!.completeError(
        const LibraryClassificationFailure("busy", "busy"),
      );
      await tester.pumpAndSettle();
      expect(find.text("确认分类"), findsNothing);
      expect(find.text("复制路径"), findsOneWidget);
      expect(
        container.read(ameNotificationControllerProvider).history.single.title,
        "暂时无法读取分类",
      );
      await tester.tap(find.text("复制路径"));
      await tester.pumpAndSettle();
      service.pendingOpen = null;
      await open(tester);
      expect(find.text("确认分类"), findsOneWidget);
      expect(
        container
            .read(ameNotificationControllerProvider)
            .history
            .single
            .isActive,
        isFalse,
      );
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
