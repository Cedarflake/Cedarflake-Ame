import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/app/notifications/ame_notification_strings.dart";
import "package:cedarflake_ame/app/presentation/ame_notification_details.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "open details follow the current condition and retire its action",
    (tester) async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final controller = container.read(
        ameNotificationControllerProvider.notifier,
      );
      controller.publish(_draft("访问失败", detail: "旧原因"));
      final observed = container
          .read(ameNotificationControllerProvider)
          .history
          .single;
      final actions = <AmeNotificationEntry>[];
      await _open(tester, container, observed, actions.add);
      expect(find.text("旧原因"), findsOneWidget);
      expect(find.text("更新图库"), findsOneWidget);

      controller.publish(_draft("核对未完成", detail: "当前原因"));
      await tester.pump();
      expect(find.text("旧原因"), findsNothing);
      expect(find.text("当前原因"), findsOneWidget);
      expect(find.text("核对未完成"), findsOneWidget);

      controller.resolve("root-a:1");
      await tester.pump();
      expect(
        find.text(AmeNotificationStrings.historicalNotificationDetail),
        findsOneWidget,
      );
      expect(find.text("更新图库"), findsNothing);
      expect(find.textContaining("已持续"), findsNothing);
      expect(actions, isEmpty);

      controller.publish(_draft("新的问题", detail: "新一轮原因"));
      await tester.pump();
      expect(find.text("核对未完成"), findsOneWidget);
      expect(find.text("新一轮原因"), findsNothing);
      expect(find.text("更新图库"), findsNothing);
    },
  );

  testWidgets("an action retired before the next frame cannot be invoked", (
    tester,
  ) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final controller = container.read(
      ameNotificationControllerProvider.notifier,
    );
    controller.publish(_draft("核对未完成"));
    final observed = container
        .read(ameNotificationControllerProvider)
        .history
        .single;
    final actions = <AmeNotificationEntry>[];
    await _open(tester, container, observed, actions.add);
    controller.resolve("root-a:1");
    await tester.tap(find.text("更新图库"));
    expect(actions, isEmpty);
    await tester.pump();
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(find.text("更新图库"), findsNothing);
  });

  testWidgets("an evicted notification cannot borrow a later action", (
    tester,
  ) async {
    final container = ProviderContainer();
    addTearDown(container.dispose);
    final controller = container.read(
      ameNotificationControllerProvider.notifier,
    );
    controller.publish(_draft("原问题"));
    final observed = container
        .read(ameNotificationControllerProvider)
        .history
        .single;
    final actions = <AmeNotificationEntry>[];
    await _open(tester, container, observed, actions.add);
    controller.resolve("root-a:1");
    for (var index = 0; index < ameNotificationHistoryLimit; index++) {
      controller.publish(
        AmeNotificationDraft(
          title: "信息 $index",
          message: "已完成",
          severity: AmeNotificationSeverity.info,
        ),
      );
    }
    controller.publish(_draft("新问题"));
    await tester.pump();
    expect(find.text("原问题"), findsOneWidget);
    expect(find.text("新问题"), findsNothing);
    expect(
      find.text(AmeNotificationStrings.historicalNotificationDetail),
      findsOneWidget,
    );
    expect(find.text("更新图库"), findsNothing);
    expect(find.textContaining("已持续"), findsNothing);
    expect(actions, isEmpty);
  });

  testWidgets(
    "the current admitted action is delivered once and closes details",
    (tester) async {
      final container = ProviderContainer();
      addTearDown(container.dispose);
      final controller = container.read(
        ameNotificationControllerProvider.notifier,
      );
      controller.publish(_draft("核对未完成"));
      final observed = container
          .read(ameNotificationControllerProvider)
          .history
          .single;
      final actions = <AmeNotificationEntry>[];
      await _open(tester, container, observed, actions.add);
      await tester.tap(find.text("更新图库"));
      await tester.pumpAndSettle();
      expect(actions, [observed]);
      expect(find.byType(AlertDialog), findsNothing);
    },
  );
}

AmeNotificationDraft _draft(String message, {String? detail}) =>
    AmeNotificationDraft(
      title: "更新受阻",
      message: message,
      severity: AmeNotificationSeverity.warning,
      dedupeKey: "root-a:1",
      detail: detail,
      elapsedStartedAt: DateTime.now().subtract(const Duration(minutes: 3)),
      actionId: "library.update",
      actionLabel: "更新图库",
      actionContext: "root-a",
      isPersistent: true,
    );

Future<void> _open(
  WidgetTester tester,
  ProviderContainer container,
  AmeNotificationEntry observed,
  ValueChanged<AmeNotificationEntry> onAction,
) async {
  await tester.pumpWidget(
    UncontrolledProviderScope(
      container: container,
      child: MaterialApp(
        theme: buildAmeTheme(),
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () => showAmeNotificationDetails(
              context,
              observed,
              onAction: onAction,
            ),
            child: const Text("打开详情"),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text("打开详情"));
  await tester.pumpAndSettle();
}
