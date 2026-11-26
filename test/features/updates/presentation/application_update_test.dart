import "dart:async";

import "package:cedarflake_ame/app/presentation/ame_localizations.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/features/settings/presentation/widgets/updates/application_updates_section.dart";
import "package:cedarflake_ame/features/updates/application/application_update_controller.dart";
import "package:cedarflake_ame/features/updates/application/application_update_state.dart";
import "package:cedarflake_ame/features/updates/application/ports/application_update_service.dart";
import "package:cedarflake_ame/features/updates/domain/application_update.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_ui/material_ui.dart";

import "../../../support/localized_test_app.dart";

void main() {
  for (final locale in ameSupportedLocales) {
    testWidgets("update remains usable at enlarged text in $locale", (
      tester,
    ) async {
      final fixture = _Fixture();
      final controller = ApplicationUpdateController(
        service: fixture,
        preferences: fixture,
        restarter: fixture,
        defaultChannel: ApplicationUpdateChannel.preview,
      );
      await controller.initialize();
      addTearDown(() {
        unawaited(controller.close());
        controller.dispose();
      });
      tester.view.physicalSize = const Size(800, 600);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            applicationUpdateControllerProvider.overrideWithValue(controller),
          ],
          child: LocalizedTestApp(
            locale: locale,
            theme: buildAmeTheme(),
            builder: (context, child) => MediaQuery(
              data: MediaQuery.of(
                context,
              ).copyWith(textScaler: const TextScaler.linear(1.5)),
              child: child!,
            ),
            home: const Scaffold(
              body: SingleChildScrollView(child: ApplicationUpdatesSection()),
            ),
          ),
        ),
      );
      final strings = AppLocalizations.of(
        tester.element(find.byType(ApplicationUpdatesSection)),
      );
      await tester.tap(find.byKey(const Key("application-update-check")));
      await tester.pumpAndSettle();
      expect(fixture.checks, 1);
      expect(find.textContaining("0.2.0"), findsOneWidget);
      expect(
        find.text("Improved browsing", findRichText: true),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
      await tester.tap(
        find.widgetWithText(FilledButton, strings.updateDownload),
      );
      await tester.pump(const Duration(milliseconds: 300));
      expect(fixture.downloads, 1);
      expect(find.text("3.1%"), findsOneWidget);
      final indicator = find.byType(LinearProgressIndicator);
      double displayedProgress() =>
          tester.widget<LinearProgressIndicator>(indicator).value!;
      final initialProgress = displayedProgress();
      fixture.receivedBytes = 32 * 1024 * 1024;
      await tester.pump(const Duration(milliseconds: 200));
      expect(find.text("50.0%"), findsOneWidget);
      await tester.pump(const Duration(milliseconds: 50));
      expect(displayedProgress(), greaterThan(initialProgress));
      expect(displayedProgress(), lessThan(0.5));
      await tester.pump(const Duration(milliseconds: 200));
      expect(displayedProgress(), 0.5);
      fixture.totalBytes = 128 * 1024 * 1024;
      await tester.pump(const Duration(milliseconds: 200));
      expect(find.text("25.0%"), findsOneWidget);
      await tester.pump(const Duration(milliseconds: 200));
      expect(displayedProgress(), 0.25);
      final labelRect = tester.getRect(find.text("25.0%"));
      final progressRect = tester.getRect(indicator);
      expect(labelRect.left, progressRect.left);
      await tester.tap(find.text(strings.updateLater));
      await tester.pumpAndSettle();
      expect(controller.state.phase, ApplicationUpdatePhase.downloading);
      await tester.tap(find.byKey(const Key("application-update-check")));
      await tester.pump();
      expect(displayedProgress(), 0.25);
      tester.platformDispatcher.accessibilityFeaturesTestValue =
          const FakeAccessibilityFeatures(disableAnimations: true);
      addTearDown(
        tester.platformDispatcher.clearAccessibilityFeaturesTestValue,
      );
      fixture.receivedBytes = 64 * 1024 * 1024;
      await tester.pump(const Duration(milliseconds: 200));
      expect(displayedProgress(), 0.5);
      await tester.tap(find.text(strings.updateLater));
      await tester.pumpAndSettle();
      fixture.finished.complete();
      await tester.pump(const Duration(milliseconds: 300));
      expect(controller.state.phase, ApplicationUpdatePhase.ready);
      await tester.tap(find.byKey(const Key("application-update-check")));
      await tester.pump();
      expect(fixture.restarts, 1);
      expect(tester.takeException(), isNull);
      await controller.close();
    }, timeout: const Timeout(Duration(seconds: 30)));
  }

  testWidgets(
    "update preferences are selected in the existing Settings menus",
    (tester) async {
      final fixture = _Fixture();
      final controller = ApplicationUpdateController(
        service: fixture,
        preferences: fixture,
        restarter: fixture,
        defaultChannel: ApplicationUpdateChannel.preview,
      );
      await controller.initialize();
      addTearDown(() {
        unawaited(controller.close());
        controller.dispose();
      });
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            applicationUpdateControllerProvider.overrideWithValue(controller),
          ],
          child: LocalizedTestApp(
            home: const Scaffold(body: ApplicationUpdatesSection()),
          ),
        ),
      );
      final strings = AppLocalizations.of(
        tester.element(find.byType(ApplicationUpdatesSection)),
      );
      await tester.tap(find.text(strings.updateSettings));
      await tester.pumpAndSettle();
      await tester.tap(find.text(strings.updateManual));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(
          const ValueKey("ame-choice-option-ApplicationUpdatePolicy.download"),
        ),
      );
      await tester.pumpAndSettle();
      expect(fixture.saved?.policy, ApplicationUpdatePolicy.download);
      await tester.tap(find.text(strings.updatePreview));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(
          const ValueKey("ame-choice-option-ApplicationUpdateChannel.stable"),
        ),
      );
      await tester.pumpAndSettle();
      expect(fixture.saved?.channel, ApplicationUpdateChannel.stable);
      expect(fixture.saved?.policy, ApplicationUpdatePolicy.download);
      expect(tester.takeException(), isNull);
      await controller.close();
    },
  );
}

class _Fixture
    implements
        ApplicationUpdateService,
        ApplicationUpdateDownload,
        ApplicationUpdatePreferenceStore,
        ApplicationUpdateRestarter {
  final finished = Completer<void>();
  int checks = 0;
  int downloads = 0;
  int restarts = 0;
  bool cancelled = false;
  bool ready = false;
  int receivedBytes = 2 * 1024 * 1024;
  int totalBytes = 64 * 1024 * 1024;
  ApplicationUpdatePreferences? saved;
  @override
  Future<bool> isSupported() async => true;
  @override
  Future<ApplicationUpdateResult?> takeResult() async => null;
  @override
  Future<ApplicationUpdateDownload?> check(
    ApplicationUpdateChannel channel,
  ) async {
    checks++;
    return this;
  }

  @override
  ApplicationRelease get release => const ApplicationRelease(
    version: "0.2.0",
    build: 5,
    downloadBytes: 64 * 1024 * 1024,
    notes: "## Changes\n\n- Improved browsing\n- Faster startup",
  );
  @override
  ApplicationDownloadProgress get progress => ApplicationDownloadProgress(
    phase: cancelled
        ? ApplicationDownloadPhase.cancelled
        : ready
        ? ApplicationDownloadPhase.ready
        : ApplicationDownloadPhase.downloading,
    receivedBytes: ready ? release.downloadBytes : receivedBytes,
    totalBytes: totalBytes,
  );
  @override
  Future<void> download() async {
    downloads++;
    await finished.future;
    ready = !cancelled;
  }

  @override
  void cancel() {
    cancelled = true;
    if (!finished.isCompleted) finished.complete();
  }

  @override
  Future<void> verify() async {}
  @override
  Future<void> prepareHandoff() async {}
  @override
  Future<void> applyAfterExit() async {}
  @override
  Future<void> close() async {}
  @override
  Future<void> restart(ApplicationUpdateDownload download) async {
    restarts++;
  }

  @override
  Future<ApplicationUpdatePreferences> load({
    required ApplicationUpdateChannel defaultChannel,
  }) async => ApplicationUpdatePreferences(
    channel: defaultChannel,
    policy: ApplicationUpdatePolicy.manual,
  );
  @override
  Future<void> save(ApplicationUpdatePreferences preferences) async {
    saved = preferences;
  }
}
