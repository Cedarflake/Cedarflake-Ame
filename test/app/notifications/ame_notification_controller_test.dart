import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "resolved conditions retain history without timers or stale actions",
    () {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final controller = container.read(
        ameNotificationControllerProvider.notifier,
      );
      controller.publish(
        AmeNotificationDraft(
          title: "更新受阻",
          message: "无法接续旧更新记录。",
          severity: AmeNotificationSeverity.warning,
          dedupeKey: "root-a:1",
          elapsedStartedAt: DateTime.utc(2026, 10, 14),
          actionId: "update-root",
          actionLabel: "更新图库",
          actionContext: "root-a",
          sourcePath: r"C:\Generated",
          isPersistent: true,
        ),
      );
      final observed = container
          .read(ameNotificationControllerProvider)
          .history
          .single;
      controller.dismiss(observed.id);
      expect(controller.admitAction(observed), isNotNull);
      expect(controller.resolve("root-a:1"), isTrue);
      final resolved = container
          .read(ameNotificationControllerProvider)
          .history
          .single;
      expect(resolved.isResolvedCondition, isTrue);
      expect(resolved.message, observed.message);
      expect(resolved.elapsedStartedAt, isNull);
      expect(resolved.actionId, isNull);
      expect(resolved.actionLabel, isNull);
      expect(controller.admitAction(observed), isNull);
    },
  );

  test("a changed or retired notification cannot authorize an old action", () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final controller = container.read(
      ameNotificationControllerProvider.notifier,
    );
    controller.publish(
      const AmeNotificationDraft(
        title: "需要更新",
        message: "",
        severity: AmeNotificationSeverity.warning,
        dedupeKey: "root-a:1",
        actionId: "update-root",
        actionLabel: "更新图库",
        sourcePath: r"C:\Old",
      ),
    );
    final observed = container
        .read(ameNotificationControllerProvider)
        .history
        .single;
    controller.publish(
      const AmeNotificationDraft(
        title: "需要更新",
        message: "",
        severity: AmeNotificationSeverity.warning,
        dedupeKey: "root-a:1",
        actionId: "update-root",
        actionLabel: "更新图库",
        sourcePath: r"C:\New",
      ),
    );
    final current = container
        .read(ameNotificationControllerProvider)
        .history
        .single;
    expect(current.id, observed.id);
    expect(controller.admitAction(observed), isNull);
    expect(controller.admitAction(current), same(current));
    controller.resolve("root-a:1");
    expect(controller.admitAction(current), isNull);
  });

  test("active notifications deduplicate until their condition resolves", () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final controller = container.read(
      ameNotificationControllerProvider.notifier,
    );
    const draft = AmeNotificationDraft(
      title: "更新受阻",
      message: "检测到无法确认的文件变化。",
      severity: AmeNotificationSeverity.warning,
      dedupeKey: "root-a:evidence-gap",
      isPersistent: true,
    );

    final id = controller.publish(
      draft,
      occurredAt: DateTime.utc(2026, 8, 20, 10),
    );
    controller.markAllRead();
    controller.dismiss(id);
    controller.publish(
      const AmeNotificationDraft(
        title: "更新受阻",
        message: "检测到无法确认的文件变化。",
        detail: "3 项等待重试",
        elapsedStartedAt: null,
        severity: AmeNotificationSeverity.warning,
        dedupeKey: "root-a:evidence-gap",
        isPersistent: true,
      ),
      occurredAt: DateTime.utc(2026, 8, 20, 10, 1),
    );

    final deduplicated = container.read(ameNotificationControllerProvider);
    expect(deduplicated.history, hasLength(1));
    expect(deduplicated.history.single.detail, "3 项等待重试");
    expect(deduplicated.pendingIds, isEmpty);
    expect(deduplicated.hasUnread, isFalse);

    expect(controller.resolve("root-a:evidence-gap"), isTrue);
    controller.publish(draft, occurredAt: DateTime.utc(2026, 8, 20, 10, 2));

    final reactivated = container.read(ameNotificationControllerProvider);
    expect(reactivated.history, hasLength(2));
    expect(reactivated.pendingIds, hasLength(1));
    expect(reactivated.hasUnread, isTrue);
  });

  test("queue order, unread state, and history remain independent", () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final controller = container.read(
      ameNotificationControllerProvider.notifier,
    );

    final firstId = controller.publish(
      const AmeNotificationDraft(
        title: "第一条",
        message: "",
        severity: AmeNotificationSeverity.info,
      ),
    );
    controller.publish(
      const AmeNotificationDraft(
        title: "第二条",
        message: "",
        severity: AmeNotificationSeverity.success,
      ),
    );

    expect(
      container.read(ameNotificationControllerProvider).current?.title,
      "第一条",
    );
    controller.markAllRead();
    expect(
      container.read(ameNotificationControllerProvider).hasUnread,
      isFalse,
    );
    controller.dismiss(firstId);

    final state = container.read(ameNotificationControllerProvider);
    expect(state.current?.title, "第二条");
    expect(state.history.map((entry) => entry.title), ["第二条", "第一条"]);
  });

  test("notification history and pending presentation stay bounded", () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final controller = container.read(
      ameNotificationControllerProvider.notifier,
    );

    for (var index = 0; index < ameNotificationHistoryLimit + 5; index += 1) {
      controller.publish(
        AmeNotificationDraft(
          title: "通知 $index",
          message: "",
          severity: AmeNotificationSeverity.info,
        ),
      );
    }

    final state = container.read(ameNotificationControllerProvider);
    expect(state.history, hasLength(ameNotificationHistoryLimit));
    expect(state.pendingIds, hasLength(ameNotificationHistoryLimit));
    expect(state.history.first.title, "通知 104");
    expect(state.history.last.title, "通知 5");
    expect(state.current?.title, "通知 5");
  });

  test("an evicted active condition can publish a fresh notification", () {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final controller = container.read(
      ameNotificationControllerProvider.notifier,
    );
    const activeDraft = AmeNotificationDraft(
      title: "更新受阻",
      message: "目录状态尚未确认。",
      severity: AmeNotificationSeverity.warning,
      dedupeKey: "root-a:unknown",
      isPersistent: true,
    );

    controller.publish(activeDraft);
    for (var index = 0; index < ameNotificationHistoryLimit; index += 1) {
      controller.publish(
        AmeNotificationDraft(
          title: "通知 $index",
          message: "",
          severity: AmeNotificationSeverity.info,
        ),
      );
    }
    controller.publish(activeDraft);

    final state = container.read(ameNotificationControllerProvider);
    expect(state.history.first.title, "更新受阻");
    expect(
      state.history.where((entry) => entry.dedupeKey == "root-a:unknown"),
      hasLength(1),
    );
    expect(state.pendingIds, contains(state.history.first.id));
  });
}
