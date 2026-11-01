import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_classification_actions.dart";
import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_classification_selection.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_classification_review_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  test(
    "failed gallery refresh retains the committed operation's undo",
    () async {
      final service = ClassificationReviewFixture();
      final library = OrganizationLibraryFixture()..refreshSucceeds = false;
      final container = ProviderContainer(
        overrides: [
          libraryClassificationReviewServiceProvider.overrideWithValue(service),
          libraryControllerProvider.overrideWith(() => library),
        ],
      );
      addTearDown(container.dispose);
      final actions = container.read(
        libraryClassificationActionsProvider.notifier,
      );
      final selection = LibraryClassificationSelection(
        service,
        classificationScope(),
      );
      addTearDown(selection.dispose);
      expect(
        await actions.apply(selection, const LibraryConfirmClassification()),
        isTrue,
      );
      await Future<void>.delayed(Duration.zero);
      final undo = container.read(libraryClassificationActionsProvider).notice!;
      final refresh = container
          .read(ameNotificationControllerProvider)
          .current!;
      expect(undo.actionId, "classification.undo");
      expect(refresh.actionId, "classification.refresh");
      library.refreshSucceeds = true;
      actions.handleNotification(refresh);
      await Future<void>.delayed(Duration.zero);
      expect(
        container.read(libraryClassificationActionsProvider).notice,
        same(undo),
      );
      expect(container.read(ameNotificationControllerProvider).current, isNull);
      actions.handleNotification(undo);
      await Future<void>.delayed(Duration.zero);
      expect(service.undos, [undo.actionContext]);
    },
  );
  test(
    "all-matching exclusions remain bounded and preparation is reused",
    () async {
      final service = ClassificationReviewFixture();
      final scope = classificationScope(
        all: true,
        ids: List.generate(700, (index) => "$index"),
      );
      final session = LibraryClassificationSelection(service, scope);
      await session.prepare();
      await session.prepare();
      final draft = service.drafts.single;
      expect(service.scopes.single.allMatching, isTrue);
      expect(draft.chunks.map((chunk) => chunk.length), [256, 256, 188]);
      expect(draft.preparations, 1);
      session.dispose();
      await Future<void>.delayed(Duration.zero);
      expect(draft.closes, 1);
    },
  );

  test(
    "retirement before open completes closes the late draft without preparing",
    () async {
      final service = ClassificationReviewFixture()
        ..pendingOpen = Completer<LibraryClassificationDraft>();
      final session = LibraryClassificationSelection(
        service,
        classificationScope(),
      );
      final preparing = session.prepare();
      final rejected = expectLater(
        preparing,
        throwsA(isA<LibraryClassificationFailure>()),
      );
      session.dispose();
      final draft = ClassificationDraftFixture(service);
      service.pendingOpen!.complete(draft);
      await rejected;
      expect(draft.preparations, 0);
      expect(draft.closes, 1);
    },
  );

  test(
    "only a committed receipt publishes undo and refreshes the gallery",
    () async {
      final service = ClassificationReviewFixture()
        ..pendingSave = Completer<LibraryClassificationReceipt>();
      final library = OrganizationLibraryFixture();
      final container = ProviderContainer(
        overrides: [
          libraryClassificationReviewServiceProvider.overrideWithValue(service),
          libraryControllerProvider.overrideWith(() => library),
        ],
      );
      addTearDown(container.dispose);
      final actions = container.read(
        libraryClassificationActionsProvider.notifier,
      );
      final selection = LibraryClassificationSelection(
        service,
        classificationScope(),
      );
      addTearDown(selection.dispose);
      final saving = actions.apply(
        selection,
        const LibraryConfirmClassification(),
      );
      await Future<void>.delayed(Duration.zero);
      expect(
        container.read(libraryClassificationActionsProvider).notice,
        isNull,
      );
      expect(library.refreshes, 0);
      final operationId = service.drafts.single.operationIds.single;
      service.pendingSave!.complete(
        LibraryClassificationReceipt(operationId, 2),
      );
      expect(await saving, isTrue);
      final notice = container
          .read(libraryClassificationActionsProvider)
          .notice!;
      expect(notice.actionContext, operationId);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      expect(library.refreshes, 1);
      actions.handleNotification(notice);
      await Future<void>.delayed(Duration.zero);
      expect(service.undos, [operationId]);
      actions.handleNotification(notice);
      await Future<void>.delayed(Duration.zero);
      expect(service.undos, hasLength(1));
      expect(
        container.read(libraryClassificationActionsProvider).notice?.title,
        "已撤销分类更改",
      );
    },
  );
}
