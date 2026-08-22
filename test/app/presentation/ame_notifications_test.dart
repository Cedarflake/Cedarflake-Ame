import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/app/presentation/ame_notifications.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
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
    final notification = _notification(
      isUnread: false,
      elapsedStartedAt: DateTime.now().subtract(
        const Duration(minutes: 2, seconds: 3),
      ),
    );

    await tester.pumpWidget(
      MaterialApp(
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
    );

    await tester.tap(find.text("打开详情"));
    await tester.pumpAndSettle();

    expect(find.textContaining("已持续 2 分"), findsOneWidget);
  });
}

AmeNotificationEntry _notification({
  required bool isUnread,
  bool isPersistent = false,
  String? actionLabel,
  DateTime? elapsedStartedAt,
}) {
  return AmeNotificationEntry(
    id: "notification-1",
    title: "“Documents”需要重新核对",
    message: "检测到无法确认的文件变化，Ame 正在自动重新核对该目录。",
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
  );
}
