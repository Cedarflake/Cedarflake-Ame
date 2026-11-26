import "dart:io";

import "package:cedarflake_ame/app/presentation/ame_feedback_card.dart";
import "package:cedarflake_ame/app/presentation/ame_feedback_style.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/app/presentation/ame_transient_feedback.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_notice.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../support/localized_test_app.dart";

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
    testWidgets("bottom card preserves layout and actions in $brightness", (
      tester,
    ) async {
      var retries = 0;
      final working = ValueNotifier(true);
      addTearDown(working.dispose);
      final theme = buildAmeTheme(brightness: brightness);
      await tester.pumpWidget(
        LocalizedTestApp(
          theme: theme,
          home: Scaffold(
            body: Center(
              child: ValueListenableBuilder<bool>(
                valueListenable: working,
                builder: (context, active, child) => AmeFeedbackCard(
                  title: "更新受阻",
                  message: "这个文件夹暂时无法访问，请重试。",
                  detail: "已保留现有图片",
                  severity: AmeNotificationSeverity.error,
                  actions: [
                    AmeFeedbackAction(label: "重试", onPressed: () => retries++),
                  ],
                  progress: active
                      ? const LinearProgressIndicator(value: 0.5)
                      : null,
                ),
              ),
            ),
          ),
        ),
      );
      final card = find.byType(AmeFeedbackCard);
      await tester.tap(find.text("重试"));
      expect(retries, 1);
      final workingHeight = tester.getSize(card).height;
      working.value = false;
      await tester.pump();
      expect(tester.getSize(card).height, workingHeight);
      await tester.pump(const Duration(milliseconds: 100));
      final retiringHeight = tester.getSize(card).height;
      await tester.pumpAndSettle();
      expect(
        retiringHeight,
        inExclusiveRange(tester.getSize(card).height, workingHeight),
      );
      expect(tester.takeException(), isNull);
    });

    testWidgets(
      "transient feedback remains until acknowledgement in $brightness",
      (tester) async {
        await tester.pumpWidget(
          LocalizedTestApp(
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
        await tester.tap(find.text("知道了"));
        await tester.pumpAndSettle();
        expect(card, findsNothing);
        expect(tester.takeException(), isNull);
      },
    );

    testWidgets(
      "viewer notice keeps its action available at enlarged text in $brightness",
      (tester) async {
        var retried = false;
        await tester.pumpWidget(
          LocalizedTestApp(
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
        await tester.tap(find.text("重试"));
        expect(retried, isTrue);
        expect(tester.takeException(), isNull);
      },
    );
  }
}
