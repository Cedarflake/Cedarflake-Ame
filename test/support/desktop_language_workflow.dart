import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/app/presentation/ame_localizations.dart";
import "package:cedarflake_ame/app/presentation/ame_selection_checkbox.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/features/library/presentation/library_synchronization_notifications.dart";
import "package:cedarflake_ame/features/library/presentation/pages/unified_library_screen.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:cedarflake_ame/features/settings/domain/ame_preferences.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_ui/material_ui.dart";

Future<void> verifyDesktopLanguageWorkflow(WidgetTester tester) async {
  final screen = find.byType(UnifiedLibraryScreen);
  final container = ProviderScope.containerOf(tester.element(screen));
  final initial = container.read(libraryControllerProvider);
  final asset = initial.assets.first;
  final root = initial.roots.firstWhere((root) => root.id == asset.rootId);
  final notifications = container.read(
    ameNotificationControllerProvider.notifier,
  );
  final condition = LibraryRootSynchronizationStatus(
    rootId: root.id,
    rootGeneration: BigInt.one,
    availability: LibraryRootAvailability.available,
    freshness: LibraryCatalogFreshness.needsReconciliation,
    freshnessCause: LibraryCatalogFreshnessCause.evidenceGap,
    continuity: LibraryContinuityState.recoveryRequired,
    phase: LibrarySynchronizationPhase.blocked,
    phaseStartedAt: DateTime.now(),
    sourceStatus: LibraryChangeSourceStatus.healthy,
    pendingChangeCount: BigInt.one,
    retryWaitCount: BigInt.zero,
    freshnessUnknownCount: BigInt.zero,
    recoveryBlocked: true,
    lastIssueCode: "live_gap_v30_explicit_recovery_required",
  );
  LibrarySynchronizationNotifications(
    notifications: notifications,
    readRoots: () => initial.roots,
  ).accept(
    LibrarySynchronizationSnapshot(
      isRunning: true,
      catalogRevision: initial.catalogRevision!,
      appliedMutationCount: 0,
      roots: {root.id: condition},
    ),
  );
  final observed = container
      .read(ameNotificationControllerProvider)
      .history
      .first;
  expect(observed.sourcePath, root.displayPath);
  notifications.dismiss(observed.id);
  await tester.pumpAndSettle();
  final checkbox = find.byKey(ValueKey("select-${asset.locationId}"));
  await tester.tap(find.byKey(const Key("library-select-button")));
  await tester.pumpAndSettle();
  await tester.tap(checkbox);
  await tester.pumpAndSettle();
  expect(tester.widget<AmeSelectionCheckbox>(checkbox).value, isTrue);

  Future<void> chooseLanguage(AmeLanguagePreference language) async {
    await tester.tap(find.byKey(const Key("library-sidebar-settings")));
    await tester.pumpAndSettle();
    final choice = find.byKey(const Key("ame-language-choice"));
    await tester.ensureVisible(choice);
    await tester.tap(choice);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(ValueKey("ame-choice-option-$language")));
    await tester.pumpAndSettle();
    expect(container.read(amePreferencesControllerProvider).language, language);
    expect(
      (await container.read(amePreferenceStoreProvider).loadAmePreferences())
          .language,
      language,
    );
  }

  await chooseLanguage(AmeLanguagePreference.en);
  expect(find.text("App language"), findsOneWidget);
  expect(Localizations.localeOf(tester.element(screen)), ameEnglishLocale);
  await tester.tap(
    find.byKey(const Key("workbench-gallery")).hitTestable().first,
  );
  await tester.pumpAndSettle();
  expect(tester.widget<AmeSelectionCheckbox>(checkbox).value, isTrue);
  expect(
    container.read(libraryControllerProvider).assets.first.displayPath,
    asset.displayPath,
  );
  await tester.tap(find.byKey(const Key("notification-history-button")));
  await tester.pumpAndSettle();
  final row = find.byKey(Key("notification-history-item-${observed.id}"));
  expect(
    find.descendant(of: row, matching: find.textContaining("blocked")),
    findsOneWidget,
  );
  await tester.tap(row);
  await tester.pumpAndSettle();
  final detail = find.byType(AlertDialog);
  expect(
    find.descendant(of: detail, matching: find.text(root.displayPath)),
    findsOneWidget,
  );
  await tester.tap(
    find.descendant(of: detail, matching: find.text("Update library")),
  );
  await tester.pumpAndSettle();
  final update = find.byKey(const Key("library-update-dialog"));
  final selectedFolder = find.descendant(
    of: update,
    matching: find.byKey(ValueKey("library-update-root-${root.id}")),
  );
  expect(tester.widget<CheckboxListTile>(selectedFolder).value, isTrue);
  await tester.tap(find.descendant(of: update, matching: find.text("Cancel")));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key("library-cancel-selection")));
  await tester.pumpAndSettle();
  await chooseLanguage(AmeLanguagePreference.zhHans);
  expect(find.text("应用语言"), findsOneWidget);
  await tester.tap(
    find.byKey(const Key("workbench-gallery")).hitTestable().first,
  );
  await tester.pumpAndSettle();
  notifications.resolve(observed.dedupeKey!);
}
