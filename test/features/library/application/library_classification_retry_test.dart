import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_classification_retry.dart";
import "package:cedarflake_ame/features/library/application/library_classification_retry_actions.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_operation_notice.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_review.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_classification_retry_fixture.dart";
import "../support/library_organization_fixture.dart";

void main() {
  late ClassificationRetryFixture retry;
  late OrganizationLibraryFixture library;
  late ProviderContainer container;
  setUp(() {
    retry = ClassificationRetryFixture();
    library = OrganizationLibraryFixture();
    container = ProviderContainer(
      overrides: [
        libraryClassificationRetryProvider.overrideWithValue(retry),
        libraryControllerProvider.overrideWith(() => library),
      ],
    );
  });
  tearDown(() => container.dispose());

  test(
    "paused retry uses shared bottom and top notices without history or undo",
    () async {
      retry.outcome = LibraryClassificationRetryOutcome.queuedWhilePaused;
      final target = organizationTarget("one");
      final actions = container.read(
        libraryClassificationRetryActionsProvider.notifier,
      );
      expect(await actions.retry(target, BigInt.from(7)), isTrue);
      expect(retry.targets, [target]);
      expect(retry.revisions, [BigInt.from(7)]);
      final notice = container
          .read(libraryClassificationRetryActionsProvider)
          .notice!;
      expect(notice.title, "已安排重试，继续分析后处理");
      expect(notice.actionId, isNull);
      expect(container.read(libraryOperationOutcomeProvider), same(notice));
      expect(
        container.read(libraryViewerOperationNoticeProvider),
        same(notice),
      );
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      expect(library.refreshes, 1);
      actions.dismiss(notice.id);
      expect(container.read(libraryViewerOperationNoticeProvider), isNull);
    },
  );

  test(
    "duplicate clicks join no second mutation and stale settings leave a retryable failure",
    () async {
      retry.pending = Completer();
      final actions = container.read(
        libraryClassificationRetryActionsProvider.notifier,
      );
      final first = actions.retry(organizationTarget("one"), BigInt.from(7));
      expect(
        await actions.retry(organizationTarget("two"), BigInt.from(7)),
        isFalse,
      );
      retry.pending!.completeError(
        const LibraryClassificationFailure(
          "classification_dispatch_changed",
          "changed",
        ),
      );
      expect(await first, isFalse);
      expect(retry.targets, hasLength(1));
      expect(library.refreshes, 0);
      final state = container.read(libraryClassificationRetryActionsProvider);
      expect(state.busy, isFalse);
      expect(state.notice!.title, "图片或分析设置已变化，请重新打开菜单后重试");
      retry.pending = null;
      expect(
        await actions.retry(organizationTarget("one"), BigInt.from(8)),
        isTrue,
      );
      expect(
        container.read(libraryClassificationRetryActionsProvider).notice!.title,
        "已安排重新分析",
      );
    },
  );

  test(
    "refresh retry reads the gallery without repeating the analysis request",
    () async {
      library.refreshSucceeds = false;
      final actions = container.read(
        libraryClassificationRetryActionsProvider.notifier,
      );
      expect(
        await actions.retry(organizationTarget("one"), BigInt.from(7)),
        isTrue,
      );
      final notice = container
          .read(libraryClassificationRetryActionsProvider)
          .notice!;
      expect(notice.actionId, "classification-analysis.refresh");
      library.refreshSucceeds = true;
      expect(actions.handleNotification(notice), isTrue);
      await Future<void>.delayed(Duration.zero);
      expect(retry.targets, hasLength(1));
      expect(library.refreshes, 2);
      expect(
        container.read(libraryClassificationRetryActionsProvider).notice!.title,
        "图库已刷新",
      );
      actions.handleNotification(notice);
      expect(library.refreshes, 2);
    },
  );

  test("retired provider does not publish a late retry result", () async {
    retry.pending = Completer();
    final actions = container.read(
      libraryClassificationRetryActionsProvider.notifier,
    );
    final pending = actions.retry(organizationTarget("one"), BigInt.from(7));
    container.invalidate(libraryClassificationRetryActionsProvider);
    retry.pending!.complete(LibraryClassificationRetryOutcome.queued);
    expect(await pending, isTrue);
    expect(
      container.read(libraryClassificationRetryActionsProvider).notice,
      isNull,
    );
    expect(library.refreshes, 0);
  });
}
