import "dart:async";

import "package:cedarflake_ame/app/presentation/ame_localizations.dart";
import "package:cedarflake_ame/app/presentation/ame_feedback_card.dart";
import "package:cedarflake_ame/features/diagnostics/application/diagnostics_controller.dart";
import "package:cedarflake_ame/features/diagnostics/application/ports/diagnostic_export.dart";
import "package:cedarflake_ame/features/diagnostics/domain/diagnostic_report.dart";
import "package:cedarflake_ame/features/diagnostics/presentation/diagnostic_feedback.dart";
import "package:cedarflake_ame/features/settings/presentation/widgets/diagnostics_settings_row.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_ui/material_ui.dart";

import "../../../support/localized_test_app.dart";

void main() {
  for (final locale in ameSupportedLocales) {
    testWidgets("exports and recovers the save destination in $locale", (
      tester,
    ) async {
      final fixture = _Fixture();
      final controller = DiagnosticsController(
        collector: fixture,
        platform: fixture,
      );
      await _mount(tester, controller, locale);
      final strings = AppLocalizations.of(
        tester.element(find.byType(DiagnosticsSettingsRow)),
      );
      fixture.destination = null;
      await tester.tap(find.byKey(const Key("diagnostics-export")));
      await tester.pumpAndSettle();
      expect(find.byType(AmeFeedbackCard), findsNothing);
      fixture.destination = "report.zip";
      fixture.failWrite = true;
      await tester.tap(find.byKey(const Key("diagnostics-export")));
      await tester.pumpAndSettle();
      expect(find.text(strings.diagnosticsFailed), findsOneWidget);
      expect(find.textContaining("OS:5"), findsOneWidget);
      fixture.failWrite = false;
      await tester.tap(find.text(strings.diagnosticsChooseAgain));
      await tester.pumpAndSettle();
      expect(fixture.pickers, 3);
      expect(find.text(strings.diagnosticsSaved), findsOneWidget);
      await tester.tap(find.text(strings.openLocation));
      await tester.pump();
      expect(fixture.opened, "report.zip");
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      await controller.close();
      controller.dispose();
    });
  }

  testWidgets("delays progress and keeps the visible cancel action usable", (
    tester,
  ) async {
    final fixture = _Fixture()..collection = Completer<DiagnosticSnapshot>();
    final controller = DiagnosticsController(
      collector: fixture,
      platform: fixture,
    );
    await _mount(tester, controller, ameChineseLocale);
    await tester.tap(find.byKey(const Key("diagnostics-export")));
    await tester.pump();
    expect(find.text(testChinese.cancel).hitTestable(), findsNothing);
    await tester.pump(const Duration(milliseconds: 251));
    await tester.pump(const Duration(milliseconds: 300));
    expect(find.text(testChinese.cancel).hitTestable(), findsOneWidget);
    await tester.tap(find.text(testChinese.cancel));
    await tester.pumpAndSettle();
    expect(find.text(testChinese.diagnosticsCancelled), findsOneWidget);
    fixture.collection!.complete(_snapshot());
    await tester.pump();
    expect(fixture.writes, 0);
    await tester.pumpWidget(const SizedBox());
    await controller.close();
    controller.dispose();
  });
}

Future<void> _mount(
  WidgetTester tester,
  DiagnosticsController controller,
  Locale locale,
) async {
  await tester.pumpWidget(
    ProviderScope(
      overrides: [diagnosticsControllerProvider.overrideWithValue(controller)],
      child: LocalizedTestApp(
        locale: locale,
        builder: (context, child) => MediaQuery(
          data: MediaQuery.of(
            context,
          ).copyWith(textScaler: const TextScaler.linear(1.5)),
          child: child!,
        ),
        home: Scaffold(
          body: Consumer(
            builder: (context, ref, _) => Column(
              children: [
                const DiagnosticsSettingsRow(),
                const Spacer(),
                ?buildDiagnosticFeedback(context, ref),
              ],
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

DiagnosticSnapshot _snapshot() =>
    DiagnosticSnapshot(capturedAt: DateTime.utc(2026), facts: []);

class _Fixture implements DiagnosticCollector, DiagnosticExportPlatform {
  String? destination = "report.zip";
  String? opened;
  bool failWrite = false;
  int pickers = 0;
  int writes = 0;
  Completer<DiagnosticSnapshot>? collection;

  @override
  Future<String?> chooseDestination(DiagnosticSaveLabels labels) async {
    pickers++;
    return destination;
  }

  @override
  Future<DiagnosticSnapshot> collect() async =>
      collection?.future ?? _snapshot();

  @override
  Future<void> openFolder(String destination) async => opened = destination;

  @override
  Future<void> write(
    String destination,
    DiagnosticSnapshot snapshot,
    DiagnosticCancellation cancellation,
  ) async {
    writes++;
    if (failWrite) {
      throw DiagnosticFailure(
        DiagnosticFailureKind.writing,
        nativeCode: DiagnosticNativeCode(DiagnosticCodeNamespace.os, "5"),
      );
    }
  }
}
