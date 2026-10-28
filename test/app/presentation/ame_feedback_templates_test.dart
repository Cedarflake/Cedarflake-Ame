import "dart:io";

import "package:cedarflake_ame/app/presentation/ame_feedback_card.dart";
import "package:cedarflake_ame/app/presentation/ame_feedback_style.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/app/presentation/ame_transient_feedback.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_notice.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

void main() {
  test("only the shared transient transport constructs snackbars", () {
    final offenders = <String>[];
    for (final file in Directory(
      "lib",
    ).listSync(recursive: true, followLinks: false).whereType<File>()) {
      final path = file.path.replaceAll(r"\", "/");
      if (!path.endsWith(".dart") ||
          path.startsWith("lib/src/rust/") ||
          path == "lib/app/presentation/ame_transient_feedback.dart") {
        continue;
      }
      if (RegExp(r"\bSnackBar\s*\(").hasMatch(file.readAsStringSync())) {
        offenders.add(path);
      }
    }
    expect(
      offenders,
      isEmpty,
      reason: "Use showAmeTransientFeedback or the existing notification host.",
    );
  });

  for (final brightness in Brightness.values) {
    testWidgets(
      "single-line feedback aligns content and actions in $brightness",
      (tester) async {
        await tester.pumpWidget(
          MaterialApp(
            theme: buildAmeTheme(brightness: brightness),
            home: Scaffold(
              body: Center(
                child: AmeFeedbackCard(
                  title: "已加入收藏夹",
                  severity: AmeNotificationSeverity.success,
                  actions: [
                    AmeFeedbackAction(label: "撤销", onPressed: () {}),
                    AmeFeedbackAction(label: "知道了", onPressed: () {}),
                  ],
                ),
              ),
            ),
          ),
        );
        final contentCenter = tester.getCenter(find.text("已加入收藏夹")).dy;
        for (final element in [
          find.byIcon(Symbols.check_circle_rounded),
          find.text("撤销"),
          find.text("知道了"),
        ]) {
          expect(tester.getCenter(element).dy, closeTo(contentCenter, 0.5));
        }
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets("bottom card preserves layout and actions in $brightness", (
      tester,
    ) async {
      var retries = 0;
      final theme = buildAmeTheme(brightness: brightness);
      await tester.pumpWidget(
        MaterialApp(
          theme: theme,
          home: Scaffold(
            body: Center(
              child: AmeFeedbackCard(
                title: "更新受阻",
                message: "这个文件夹暂时无法访问，请重试。",
                detail: "已保留现有图片",
                severity: AmeNotificationSeverity.error,
                actions: [
                  AmeFeedbackAction(label: "重试", onPressed: () => retries++),
                ],
                progress: const LinearProgressIndicator(value: 0.5),
              ),
            ),
          ),
        ),
      );
      final card = find.byType(AmeFeedbackCard);
      final surface = tester.widget<Material>(
        find.descendant(of: card, matching: find.byType(Material)).first,
      );
      expect(surface.color, theme.colorScheme.surfaceContainerHigh);
      expect(surface.elevation, ameNotificationElevation);
      expect(
        surface.borderRadius,
        BorderRadius.circular(ameNotificationRadius),
      );
      expect(tester.getSize(card).width, ameNotificationWidth);
      expect(
        tester.widget<Text>(find.text("更新受阻")).style,
        Theme.of(tester.element(card)).textTheme.titleSmall,
      );
      expect(
        tester.getRect(find.byType(LinearProgressIndicator)).top,
        greaterThan(tester.getRect(find.text("已保留现有图片")).bottom),
      );
      final textBlockCenter =
          (tester.getTopLeft(find.text("更新受阻")).dy +
              tester.getBottomLeft(find.text("已保留现有图片")).dy) /
          2;
      expect(
        tester.getCenter(find.byIcon(Symbols.error_rounded)).dy,
        closeTo(textBlockCenter, 0.5),
      );
      expect(
        tester.getCenter(find.text("重试")).dy,
        closeTo(textBlockCenter, 0.5),
      );
      await tester.tap(find.text("重试"));
      expect(retries, 1);
      expect(tester.takeException(), isNull);
    });

    testWidgets(
      "transient feedback shares bottom card, inset and acknowledgement in $brightness",
      (tester) async {
        await tester.pumpWidget(
          MaterialApp(
            theme: buildAmeTheme(brightness: brightness),
            home: Scaffold(
              body: Builder(
                builder: (context) => TextButton(
                  onPressed: () => showAmeTransientFeedback(
                    context,
                    message: "诊断信息已复制",
                    severity: AmeNotificationSeverity.success,
                  ),
                  child: const Text("复制"),
                ),
              ),
            ),
          ),
        );
        await tester.tap(find.text("复制"));
        await tester.pumpAndSettle();
        final card = find.byType(AmeFeedbackCard);
        expect(card, findsOneWidget);
        expect(tester.getSize(card).width, ameNotificationWidth);
        expect(tester.getBottomLeft(card).dy, 600 - ameNotificationBottomInset);
        expect(find.byIcon(Symbols.check_circle_rounded), findsOneWidget);
        await tester.tap(find.text("知道了"));
        await tester.pumpAndSettle();
        expect(card, findsNothing);
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets(
      "viewer notice retains top placement, wrapping and action in $brightness",
      (tester) async {
        var retried = false;
        await tester.pumpWidget(
          MaterialApp(
            theme: buildAmeTheme(brightness: brightness),
            home: Scaffold(
              body: MediaQuery(
                data: const MediaQueryData(textScaler: TextScaler.linear(1.5)),
                child: Stack(
                  children: [
                    LibraryViewerNotice(
                      icon: Symbols.broken_image_rounded,
                      message: "原图暂时无法打开，正在显示已有预览。原图恢复后可以重新加载。",
                      actionLabel: "重试",
                      onAction: () => retried = true,
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
        expect(find.byType(AmeFeedbackSurface), findsOneWidget);
        expect(find.byType(IconButton), findsNothing);
        expect(
          tester.getTopLeft(find.byType(AmeFeedbackSurface)).dy,
          ameViewerNoticeInset,
        );
        expect(
          tester.getSize(find.byType(AmeFeedbackSurface)).width,
          lessThanOrEqualTo(ameNotificationWidth),
        );
        await tester.tap(find.text("重试"));
        expect(retried, isTrue);
        expect(tester.takeException(), isNull);
      },
    );
  }
}
