import "dart:async";

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_actions.dart";
import "package:cedarflake_ame/features/library/application/library_organization_history.dart";
import "package:cedarflake_ame/features/library/application/library_organization_outcome.dart";
import "package:cedarflake_ame/features/library/application/library_organization_requests.dart";
import "package:cedarflake_ame/features/library/domain/library_organization.dart";
import "package:cedarflake_ame/features/library/domain/library_organization_history.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_organization_fixture.dart";

void main() {
  late OrganizationServiceFixture service;
  late OrganizationLibraryFixture library;
  late ProviderContainer container;
  late LibraryOrganizationActions actions;
  var disposed = false;

  setUp(() {
    disposed = false;
    service = OrganizationServiceFixture();
    library = OrganizationLibraryFixture();
    container = ProviderContainer(
      overrides: [
        libraryOrganizationProvider.overrideWithValue(service),
        libraryControllerProvider.overrideWith(() => library),
      ],
    );
    actions = container.read(libraryOrganizationActionsProvider.notifier);
  });
  tearDown(() {
    if (!disposed) container.dispose();
  });

  AmeNotificationEntry notice(String action) => [
    ?container.read(libraryOrganizationOutcomeProvider),
    ...container.read(ameNotificationControllerProvider).history,
  ].firstWhere((entry) => entry.actionId == action);
  Future<void> settle() async {
    await Future<void>.delayed(Duration.zero);
  }

  test(
    "publishes favorite and undo only after commit, admitting one write",
    () async {
      final committed = Completer<LibraryOrganizationReceipt>();
      service.save = (_) => committed.future;
      final save = actions.favorite(organizationTarget("one"), true);
      await settle();
      await actions.favorite(organizationTarget("two"), false);
      expect(service.writes, hasLength(1));
      expect(container.read(libraryOrganizationActionsProvider).saving, isTrue);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      expect(library.refreshes, 0);
      committed.complete(service.receipt(service.writes.single.id));
      await save;
      expect(
        container.read(libraryOrganizationActionsProvider).saving,
        isFalse,
      );
      await settle();
      expect(library.refreshes, 1);
      expect(
        notice("organization.undo").actionContext,
        service.writes.single.id,
      );
      expect(container.read(libraryOrganizationActionsProvider).sequence, 1);
      expect(
        container.read(ameNotificationControllerProvider).history,
        isEmpty,
      );
      expect(
        container.read(ameNotificationControllerProvider).hasUnread,
        isFalse,
      );
    },
  );

  test(
    "a committed action returns while publication remains pending",
    () async {
      final refresh = Completer<bool>();
      library.pendingRefresh = refresh;
      LibraryOrganizationCommandResult? result;
      final saving = actions
          .submit(LibraryFavoriteRequest(organizationTarget("one"), true))
          .then((value) => result = value);
      await settle();
      expect(result, isA<LibraryOrganizationSaved>());
      expect(library.refreshes, 1);
      expect(
        container.read(libraryOrganizationActionsProvider).saving,
        isFalse,
      );
      expect(
        notice("organization.undo").actionContext,
        service.writes.single.id,
      );

      refresh.complete(false);
      await saving;
      await settle();
      final failedRefresh = notice("organization.refresh");
      library.pendingRefresh = null;
      actions.handleNotification(failedRefresh);
      await settle();
      expect(library.refreshes, 2);
      expect(service.writes, hasLength(1));
      expect(
        container.read(ameNotificationControllerProvider).pendingIds,
        isNot(contains(failedRefresh.id)),
      );
    },
  );

  test(
    "retry after an unknown receipt keeps the operation identity and authority",
    () async {
      service.save = (_) async => throw StateError("Receipt unavailable");
      await actions.favorite(organizationTarget("one"), true);
      final failed = notice("organization.retry");
      expect(library.refreshes, 0);
      service.save = null;
      expect(actions.handleNotification(failed), isTrue);
      await settle();
      expect(service.preparationCount, 1);
      expect(service.writes, hasLength(2));
      expect(service.writes[1].id, service.writes[0].id);
      expect(service.writes[1].prepared, same(service.writes[0].prepared));
      expect(
        container.read(ameNotificationControllerProvider).pendingIds,
        isNot(contains(failed.id)),
      );
      expect(library.refreshes, 1);
    },
  );

  test(
    "confirmed stale authority is prepared again under a new operation",
    () async {
      service.save = (_) async => throw const LibraryOrganizationFailure(
        "organization_selection_changed",
        "Selection changed",
      );
      await actions.favorite(organizationTarget("one"), true);
      final failed = notice("organization.retry");
      service.save = null;
      actions.handleNotification(failed);
      await settle();
      expect(service.preparationCount, 2);
      expect(service.writes[1].id, isNot(service.writes[0].id));
      expect(
        container.read(ameNotificationControllerProvider).pendingIds,
        isNot(contains(failed.id)),
      );
    },
  );

  test(
    "refresh failure retries the read without replaying a committed favorite",
    () async {
      library.refreshSucceeds = false;
      await actions.favorite(organizationTarget("one"), true);
      await settle();
      final refresh = notice("organization.refresh");
      expect(refresh.title, "更改已保存");
      library.refreshSucceeds = true;
      actions.handleNotification(refresh);
      await settle();
      expect(service.writes, hasLength(1));
      expect(library.refreshes, 2);
      expect(
        container.read(ameNotificationControllerProvider).pendingIds,
        isNot(contains(refresh.id)),
      );
    },
  );

  test(
    "undo retires the original action and preserves a newer unavailable decision",
    () async {
      await actions.favorite(organizationTarget("one"), true);
      final original = notice("organization.undo");
      actions.handleNotification(original);
      await settle();
      expect(service.undos.single.originalId, service.writes.single.id);
      expect(service.undos.single.revision, BigInt.two);
      expect(service.isFavorite, isFalse);
      actions.handleNotification(original);
      await settle();
      expect(service.undos, hasLength(1));

      await actions.favorite(organizationTarget("one"), true);
      service.undoResult = (_) async => throw const LibraryOrganizationFailure(
        "organization_undo_unavailable",
        "A newer decision exists",
      );
      actions.handleNotification(notice("organization.undo"));
      await settle();
      expect(service.isFavorite, isTrue);
      final failure = container
          .read(ameNotificationControllerProvider)
          .history
          .first;
      expect(failure.title, "无法撤销");
      expect(failure.actionId, isNull);
    },
  );

  test("disposing pending preparation cannot dispatch a later write", () async {
    service.preparing = Completer<LibraryOrganizationPrepared>();
    final save = actions.favorite(organizationTarget("one"), true);
    await settle();
    container.dispose();
    disposed = true;
    service.preparing!.complete(const OrganizationPreparedFixture());
    await save;
    expect(service.writes, isEmpty);
    expect(library.refreshes, 0);
  });

  test("favorite outcomes stay transient and reject superseded undo", () async {
    await actions.favorite(organizationTarget("one"), true);
    final added = notice("organization.undo");
    await actions.favorite(organizationTarget("one"), false);
    final removed = notice("organization.undo");
    final outcomes = container.read(
      libraryOrganizationOutcomeProvider.notifier,
    );
    outcomes.dismiss(added.id);
    expect(container.read(libraryOrganizationOutcomeProvider), same(removed));
    actions.handleNotification(added);
    await settle();
    expect(service.undos, isEmpty);
    actions.handleNotification(removed);
    await settle();
    expect(service.undos, hasLength(1));
    final notifications = container.read(ameNotificationControllerProvider);
    expect(notifications.history, isEmpty);
    expect(notifications.pendingIds, isEmpty);
    expect(notifications.hasUnread, isFalse);
    expect(container.read(libraryOrganizationOutcomeProvider)?.title, "已撤销");
  });

  test(
    "an uncertain history result retries the same receipt and does not infer another head",
    () async {
      service.saveHistory = (_) async =>
          throw StateError("Receipt unavailable");
      await actions.submit(
        LibraryOrganizationHistoryRequest(
          direction: LibraryOrganizationHistoryDirection.redo,
          originalId: "previous-image",
          revision: BigInt.two,
        ),
      );
      final first = service.historySteps.single;
      service.saveHistory = null;
      actions.handleNotification(notice("organization.retry"));
      await settle();
      expect(service.historySteps.last, first);
      expect(container.read(libraryOrganizationOutcomeProvider)?.title, "已重做");
      expect(library.refreshes, 1);
    },
  );

  test(
    "a stale history retry cannot silently undo a different operation",
    () async {
      service.saveHistory = (_) async => throw const LibraryOrganizationFailure(
        "organization_selection_changed",
        "Changed",
      );
      await actions.submit(
        LibraryOrganizationHistoryRequest(
          direction: LibraryOrganizationHistoryDirection.undo,
          originalId: "observed-operation",
          revision: BigInt.two,
        ),
      );
      service.storedHistory = LibraryOrganizationHistory(
        revision: BigInt.from(3),
        undoOperationId: "newer-operation",
      );
      service.saveHistory = null;
      actions.handleNotification(notice("organization.retry"));
      await settle();
      expect(service.historySteps, hasLength(1));
      final current = container
          .read(ameNotificationControllerProvider)
          .history
          .first;
      expect(current.message, "操作记录已更新，请重新打开菜单。");
      expect(current.actionId, isNull);
      expect(library.refreshes, 0);
    },
  );

  test(
    "a content guard failure identifies redo and offers no impossible retry",
    () async {
      service.saveHistory = (_) async => throw const LibraryOrganizationFailure(
        "organization_undo_unavailable",
        "Content changed",
      );
      await actions.submit(
        LibraryOrganizationHistoryRequest(
          direction: LibraryOrganizationHistoryDirection.redo,
          originalId: "previous-image",
          revision: BigInt.two,
        ),
      );
      final entry = container
          .read(ameNotificationControllerProvider)
          .history
          .single;
      expect(entry.title, "无法重做");
      expect(entry.actionId, isNull);
      expect(library.refreshes, 0);
    },
  );
}
