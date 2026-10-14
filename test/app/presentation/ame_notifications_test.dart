import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/app/presentation/ame_notifications.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("resolved history retains its message without recovery actions", (
    tester,
  ) async {
    final resolved = _notification(
      isUnread: false,
      dedupeKey: "root-a:1",
      actionLabel: "更新图库",
      elapsedStartedAt: DateTime.now().subtract(const Duration(minutes: 3)),
    ).copyWith(isActive: false);
    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: Builder(
            builder: (context) => TextButton(
              onPressed: () => showAmeNotificationDetails(
                context,
                resolved,
                onAction: (_) => fail("Resolved history invoked an action"),
              ),
              child: const Text("打开详情"),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text("打开详情"));
    await tester.pumpAndSettle();
    expect(find.textContaining("已持续"), findsNothing);
    expect(find.text("更新图库"), findsNothing);
    expect(find.text(resolved.message), findsOneWidget);
  });
  testWidgets(
    "history button switches to the unread-dot icon without a count",
    (tester) async {
      var opened = 0;
      AmeNotificationEntry? selected;
      final notification = _notification(isUnread: true);

      await tester.pumpWidget(
        MaterialApp(
          theme: buildAmeTheme(),
          home: Scaffold(
            body: Align(
              alignment: Alignment.topRight,
              child: AmeNotificationHistoryButton(
                state: AmeNotificationState(history: [notification]),
                onOpened: () => opened += 1,
                onSelected: (value) => selected = value,
              ),
            ),
          ),
        ),
      );

      expect(find.byKey(const Key("notification-unread-icon")), findsOneWidget);
      expect(find.text("通知历史"), findsNothing);
      expect(find.text("1"), findsNothing);

      await tester.tap(find.byKey(const Key("notification-history-button")));
      await tester.pumpAndSettle();

      expect(opened, 1);
      expect(find.text("“Documents”需要重新核对"), findsOneWidget);
      expect(
        tester
            .getSize(
              find.byKey(Key("notification-history-item-${notification.id}")),
            )
            .width,
        360,
      );
      await tester.tap(
        find.byKey(Key("notification-history-item-${notification.id}")),
      );
      await tester.pumpAndSettle();
      expect(selected, notification);
    },
  );

  testWidgets(
    "history button uses the ordinary icon after notifications are read",
    (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          theme: buildAmeTheme(),
          home: Scaffold(
            body: AmeNotificationHistoryButton(
              state: AmeNotificationState(
                history: [_notification(isUnread: false)],
              ),
              onOpened: () {},
              onSelected: (_) {},
            ),
          ),
        ),
      );

      expect(find.byKey(const Key("notification-read-icon")), findsOneWidget);
      expect(find.byKey(const Key("notification-unread-icon")), findsNothing);
    },
  );

  testWidgets(
    "long notification history stays bounded on the shared popup route",
    (tester) async {
      const longTitle = "一个很长的通知标题用于确认弹出菜单不会因为内容长度而改变视觉与交互契约";
      const longMessage = "一个同样很长的通知说明用于确认两行截断、滚动边界和菜单宽度都保持稳定。";
      final notifications = [
        for (var index = 0; index < 18; index += 1)
          _notification(
            id: "notification-$index",
            title: "$longTitle $index",
            message: longMessage,
            isUnread: index == 0,
          ),
      ];

      await tester.pumpWidget(
        MaterialApp(
          theme: buildAmeTheme(),
          home: Scaffold(
            body: Align(
              alignment: Alignment.topRight,
              child: AmeNotificationHistoryButton(
                state: AmeNotificationState(history: notifications),
                onOpened: () {},
                onSelected: (_) {},
              ),
            ),
          ),
        ),
      );

      await tester.tap(find.byKey(const Key("notification-history-button")));
      await tester.pumpAndSettle();

      expect(find.byType(MenuAnchor), findsNothing);
      expect(
        find.byWidgetPredicate(
          (widget) => widget is AmePopupMenuButton<AmeNotificationEntry>,
        ),
        findsOneWidget,
      );
      final firstItem = find.byKey(
        const Key("notification-history-item-notification-0"),
      );
      expect(tester.getSize(firstItem).width, 360);
      final title = tester.widget<Text>(find.text("$longTitle 0"));
      expect(title.maxLines, 1);
      expect(title.overflow, TextOverflow.ellipsis);
      final message = tester.widget<Text>(find.text(longMessage).first);
      expect(message.maxLines, 2);
      expect(message.overflow, TextOverflow.ellipsis);

      final scrollable = find.byType(SingleChildScrollView);
      expect(scrollable, findsOneWidget);
      expect(tester.getSize(scrollable).height, lessThanOrEqualTo(520));
      final lastItem = find.byKey(
        const Key("notification-history-item-notification-17"),
      );
      await Scrollable.ensureVisible(
        tester.element(lastItem),
        duration: Duration.zero,
      );
      await tester.pumpAndSettle();
      expect(lastItem.hitTestable(), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    "persistent notification surface exposes action and acknowledgement",
    (tester) async {
      AmeNotificationEntry? action;
      String? dismissed;
      final notification = _notification(
        isUnread: true,
        isPersistent: true,
        actionLabel: "立即核对",
      );

      await tester.pumpWidget(
        MaterialApp(
          theme: buildAmeTheme(),
          home: Scaffold(
            body: AmeNotificationSurface(
              notification: notification,
              onAction: (value) => action = value,
              onDismiss: (value) => dismissed = value,
            ),
          ),
        ),
      );

      expect(find.text(notification.title), findsOneWidget);
      expect(find.text(notification.message), findsOneWidget);
      expect(find.text(notification.detail!), findsOneWidget);

      await tester.tap(find.byKey(const Key("notification-primary-action")));
      expect(action, notification);
      await tester.tap(find.byKey(const Key("notification-dismiss-button")));
      expect(dismissed, notification.id);
    },
  );

  testWidgets("transient notification dismisses after the bounded duration", (
    tester,
  ) async {
    String? dismissed;
    final notification = _notification(isUnread: true);

    await tester.pumpWidget(
      MaterialApp(
        theme: buildAmeTheme(),
        home: Scaffold(
          body: AmeNotificationSurface(
            notification: notification,
            onAction: (_) {},
            onDismiss: (value) => dismissed = value,
          ),
        ),
      ),
    );

    await tester.pump(const Duration(seconds: 4));
    expect(dismissed, isNull);
    await tester.pump(const Duration(seconds: 1));
    expect(dismissed, notification.id);
  });

  testWidgets("notification details calculate elapsed time when opened", (
    tester,
  ) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    container
        .read(ameNotificationControllerProvider.notifier)
        .publish(
          AmeNotificationDraft(
            title: "更新受阻",
            message: "无法访问目录",
            severity: AmeNotificationSeverity.warning,
            elapsedStartedAt: DateTime.now().subtract(
              const Duration(minutes: 2, seconds: 3),
            ),
          ),
        );
    final notification = container
        .read(ameNotificationControllerProvider)
        .history
        .single;

    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container,
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: Builder(
            builder: (context) => TextButton(
              onPressed: () => showAmeNotificationDetails(
                context,
                notification,
                onAction: (_) {},
              ),
              child: const Text("打开详情"),
            ),
          ),
        ),
      ),
    );

    await tester.tap(find.text("打开详情"));
    await tester.pumpAndSettle();

    expect(find.textContaining("已持续 2 分"), findsOneWidget);
  });
}

AmeNotificationEntry _notification({
  required bool isUnread,
  String id = "notification-1",
  String title = "“Documents”需要重新核对",
  String message = "部分文件变化尚未核对，当前仍显示上次确认的图库内容。",
  bool isPersistent = false,
  String? actionLabel,
  String? dedupeKey,
  DateTime? elapsedStartedAt,
}) {
  return AmeNotificationEntry(
    id: id,
    title: title,
    message: message,
    detail: "3 项等待重试",
    elapsedStartedAt: elapsedStartedAt,
    sourcePath: r"C:\Users\Example\Documents",
    technicalCode: "watcher_failed",
    actionId: actionLabel == null ? null : "library.reconcileRoot",
    actionLabel: actionLabel,
    actionContext: actionLabel == null ? null : r"C:\Users\Example\Documents",
    severity: AmeNotificationSeverity.warning,
    occurredAt: DateTime.utc(2026, 8, 20, 10),
    isUnread: isUnread,
    isActive: isPersistent,
    isPersistent: isPersistent,
    dedupeKey: dedupeKey,
  );
}
