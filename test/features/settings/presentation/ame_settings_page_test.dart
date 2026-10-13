import "dart:async";
import "dart:ui" show CheckedState;

import "package:cedarflake_ame/app/notifications/ame_notification_controller.dart";
import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:cedarflake_ame/features/settings/presentation/ame_settings_page.dart";
import "package:cedarflake_ame/features/settings/presentation/widgets/settings_section.dart";
import "package:cedarflake_ame/features/settings/presentation/widgets/storage_settings_section.dart";
import "package:cedarflake_ame/features/storage/application/storage_settings.dart";
import "package:cedarflake_ame/features/storage/domain/storage_models.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";

void main() {
  testWidgets("renders settings in the main canvas with plain-language rows", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(_status());
    addTearDown(gateway.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: const Scaffold(body: AmeSettingsPage(hasLibraryRoots: true)),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(
      tester.widget<Text>(find.text("设置")).style?.fontWeight,
      ameFontWeightSemibold,
    );
    expect(
      tester.widget<Text>(find.text("个性化")).style?.fontWeight,
      ameFontWeightSemibold,
    );
    expect(
      tester.widget<Text>(find.text("应用主题")).style?.fontWeight,
      ameFontWeightSemibold,
    );
    expect(
      DefaultTextStyle.of(
        tester.element(find.text("选择明暗外观，主题色跟随 Windows")),
      ).style.fontWeight,
      ameFontWeightRegular,
    );

    expect(find.byKey(const Key("ame-settings-page")), findsOneWidget);
    expect(find.byType(AlertDialog), findsNothing);
    expect(find.text("个性化"), findsOneWidget);
    expect(find.text("浏览"), findsOneWidget);
    expect(find.text("缩略图加载速度"), findsOneWidget);
    expect(find.text("存储"), findsOneWidget);
    expect(find.text("关于"), findsOneWidget);
    expect(find.text("图库数据位置"), findsOneWidget);
    expect(find.text("缩略图位置"), findsOneWidget);
    expect(find.text("已有图库时不可更改"), findsOneWidget);
    expect(find.textContaining("Settings file"), findsNothing);
    expect(
      find.textContaining(r"C:\AmeData\catalog\ame.sqlite3"),
      findsOneWidget,
    );
    expect(find.textContaining(r"\\?\C:\AmeData"), findsNothing);
    expect(
      find.byWidgetPredicate((widget) => widget is SettingsChoice<Object?>),
      findsNWidgets(5),
    );
    expect(
      find.byWidgetPredicate((widget) => widget is AmePopupMenuButton<Object?>),
      findsNWidgets(5),
    );
    expect(find.byType(MenuAnchor), findsNothing);
    expect(
      find.byWidgetPredicate((widget) => widget is DropdownMenu<Object?>),
      findsNothing,
    );
    expect(find.byIcon(Symbols.arrow_drop_down_rounded), findsNWidgets(5));
    expect(find.byIcon(Icons.arrow_drop_down), findsNothing);
    expect(find.byIcon(Icons.arrow_drop_up), findsNothing);
    expect(AmeMenuTransition.duration, const Duration(milliseconds: 200));
  });

  testWidgets(
    "shows truthful indeterminate progress and cancellation during legacy catalog conversion",
    (tester) async {
      final gateway = _FakeStorageSettingsGateway(
        _status(
          catalogUsedBytes: _gibibytes(1),
          catalogLiveBytes: BigInt.from(5 * 1024 * 1024),
          catalogReclaimableBytes: BigInt.from(1019 * 1024 * 1024),
          catalogReclamation: _catalogReclamation(
            phase: CatalogReclamationPhase.converting,
            operationId: "catalog-reclamation-test",
            liveBytes: BigInt.from(5 * 1024 * 1024),
            reclaimableBytes: BigInt.from(1019 * 1024 * 1024),
          ),
        ),
      );
      addTearDown(gateway.dispose);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            storageSettingsGatewayProvider.overrideWithValue(gateway),
          ],
          child: MaterialApp(
            theme: buildAmeTheme(),
            home: const Scaffold(body: AmeSettingsPage(hasLibraryRoots: false)),
          ),
        ),
      );
      await tester.pump();
      await tester.pump();

      final row = find.byKey(const Key("catalog-reclamation-setting"));
      await tester.ensureVisible(row);
      await tester.pump();
      expect(find.text("正在优化图库数据库"), findsOneWidget);
      expect(find.textContaining("无法可靠估算百分比"), findsOneWidget);
      final progress = tester.widget<LinearProgressIndicator>(
        find.descendant(
          of: row,
          matching: find.byType(LinearProgressIndicator),
        ),
      );
      expect(progress.value, isNull);

      await tester.tap(
        find.descendant(of: row, matching: find.text("取消")).hitTestable(),
      );
      await tester.pump();
      expect(
        gateway.lastCatalogReclamationCancellation,
        "catalog-reclamation-test",
      );
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets("cancels the operation returned by manual catalog cleanup", (
    tester,
  ) async {
    final gateway =
        _FakeStorageSettingsGateway(
            _status(
              catalogUsedBytes: _gibibytes(1),
              catalogLiveBytes: BigInt.from(5 * 1024 * 1024),
              catalogReclaimableBytes: BigInt.from(1019 * 1024 * 1024),
              catalogReclamation: _catalogReclamation(
                phase: CatalogReclamationPhase.idle,
                liveBytes: BigInt.from(5 * 1024 * 1024),
                reclaimableBytes: BigInt.from(1019 * 1024 * 1024),
              ),
            ),
          )
          ..startedCatalogReclamation = _catalogReclamation(
            phase: CatalogReclamationPhase.converting,
            operationId: "catalog-reclamation-manual",
            liveBytes: BigInt.from(5 * 1024 * 1024),
            reclaimableBytes: BigInt.from(1019 * 1024 * 1024),
          );
    addTearDown(gateway.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: const Scaffold(body: AmeSettingsPage(hasLibraryRoots: false)),
        ),
      ),
    );
    await tester.pump();
    await tester.pump();

    final row = find.byKey(const Key("catalog-reclamation-setting"));
    await tester.ensureVisible(row);
    await tester.pump();
    await tester.tap(
      find.descendant(of: row, matching: find.text("清理")).hitTestable(),
    );
    await tester.pump();
    expect(find.text("正在优化图库数据库"), findsOneWidget);

    await tester.tap(
      find.descendant(of: row, matching: find.text("取消")).hitTestable(),
    );
    await tester.pump();
    expect(
      gateway.lastCatalogReclamationCancellation,
      "catalog-reclamation-manual",
    );
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets(
    "discovers automatic catalog reclamation when the mounted root set changes",
    (tester) async {
      final gateway = _FakeStorageSettingsGateway(_status());
      addTearDown(gateway.dispose);

      Widget buildPage(Set<String> rootIds) {
        return ProviderScope(
          overrides: [
            storageSettingsGatewayProvider.overrideWithValue(gateway),
          ],
          child: MaterialApp(
            theme: buildAmeTheme(),
            home: Scaffold(
              body: AmeSettingsPage(
                hasLibraryRoots: rootIds.isNotEmpty,
                libraryRootIds: rootIds,
              ),
            ),
          ),
        );
      }

      await tester.pumpWidget(buildPage(const {"root-a", "root-b"}));
      await tester.pump();
      expect(gateway.loadCount, 1);

      gateway.status = _status(
        catalogReclamation: _catalogReclamation(
          phase: CatalogReclamationPhase.queued,
          operationId: "automatic-root-b-removal",
          reclaimableBytes: BigInt.from(32 * 1024 * 1024),
        ),
      );
      await tester.pumpWidget(buildPage(const {"root-a"}));
      await tester.pump();

      expect(gateway.loadCount, 2);
      expect(find.text("图库数据等待整理"), findsOneWidget);
      expect(
        find.descendant(
          of: find.byKey(const Key("catalog-reclamation-setting")),
          matching: find.text("取消"),
        ),
        findsOneWidget,
      );
      await tester.pumpWidget(const SizedBox.shrink());
    },
  );

  testWidgets("clears a transient catalog reclamation status error", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(
      _status(
        catalogReclamation: _catalogReclamation(
          phase: CatalogReclamationPhase.reclaiming,
          operationId: "catalog-reclamation-poll",
          reclaimableBytes: BigInt.from(32 * 1024 * 1024),
        ),
      ),
    );
    addTearDown(gateway.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: const Scaffold(body: AmeSettingsPage(hasLibraryRoots: false)),
        ),
      ),
    );
    await tester.pump();
    gateway.catalogReclamationLoadError = StateError("controlled poll failure");

    await tester.pump(const Duration(milliseconds: 500));
    await tester.pump();
    expect(find.byKey(const Key("catalog-reclamation-error")), findsOneWidget);
    expect(find.text("图库数据整理状态异常"), findsOneWidget);
    expect(find.byKey(const Key("storage-settings-error")), findsNothing);

    gateway.catalogReclamationLoadError = null;
    await tester.pump(const Duration(milliseconds: 500));
    await tester.pump();
    expect(find.byKey(const Key("catalog-reclamation-error")), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets("retries the full storage refresh after terminal load fails", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(
      _status(
        catalogUsedBytes: _gibibytes(1),
        catalogReclamation: _catalogReclamation(
          phase: CatalogReclamationPhase.reclaiming,
          operationId: "catalog-reclamation-terminal",
          reclaimableBytes: BigInt.from(512 * 1024 * 1024),
        ),
      ),
    );
    addTearDown(gateway.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: const Scaffold(body: AmeSettingsPage(hasLibraryRoots: false)),
        ),
      ),
    );
    await tester.pump();
    gateway.status = _status(
      catalogUsedBytes: BigInt.from(512 * 1024 * 1024),
      catalogReclamation: _catalogReclamation(
        phase: CatalogReclamationPhase.completed,
        operationId: "catalog-reclamation-terminal",
      ),
    );
    gateway.loadError = StateError("controlled terminal refresh failure");

    await tester.pump(const Duration(milliseconds: 500));
    await tester.pump();
    expect(gateway.loadCount, 2);
    expect(find.byKey(const Key("catalog-reclamation-error")), findsOneWidget);
    expect(find.textContaining("当前占用 1 GB"), findsOneWidget);

    gateway.loadError = null;
    final errorRow = find.byKey(const Key("catalog-reclamation-error"));
    await tester.ensureVisible(errorRow);
    await tester.pump();
    await tester.tap(
      find.descendant(of: errorRow, matching: find.text("重试")).hitTestable(),
    );
    await tester.pump();

    expect(gateway.loadCount, 3);
    expect(find.byKey(const Key("catalog-reclamation-error")), findsNothing);
    expect(find.textContaining("当前占用 512 MB"), findsOneWidget);
  });

  testWidgets("old full storage load cannot restore pre-reclamation usage", (
    tester,
  ) async {
    final oldStatus = _status(
      catalogUsedBytes: _gibibytes(1),
      catalogLiveBytes: BigInt.from(512 * 1024 * 1024),
      catalogReclaimableBytes: BigInt.from(512 * 1024 * 1024),
      catalogReclamation: _catalogReclamation(
        operationId: "operation-a",
        phase: CatalogReclamationPhase.reclaiming,
      ),
    );
    final gateway = _FakeStorageSettingsGateway(oldStatus);
    addTearDown(gateway.dispose);
    var roots = <String>{"root-a"};
    late StateSetter updateRoots;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: MaterialApp(
          home: Scaffold(
            body: SingleChildScrollView(
              child: StatefulBuilder(
                builder: (context, setState) {
                  updateRoots = setState;
                  return StorageSettingsSection(
                    hasLibraryRoots: roots.isNotEmpty,
                    libraryRootIds: roots,
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    final oldLoad = Completer<StorageStatusModel>();
    gateway.pendingStorageLoad = oldLoad.future;
    updateRoots(() => roots = {});
    await tester.pump();
    expect(gateway.loadCount, 2);
    gateway.status = _status(
      catalogUsedBytes: BigInt.from(512 * 1024 * 1024),
      catalogLiveBytes: BigInt.from(512 * 1024 * 1024),
      previewUsedBytes: BigInt.from(64 * 1024 * 1024),
      catalogReclamation: _catalogReclamation(
        operationId: "operation-a",
        phase: CatalogReclamationPhase.completed,
        liveBytes: BigInt.from(512 * 1024 * 1024),
      ),
    );
    await tester.pump(const Duration(milliseconds: 500));
    await tester.pump();
    expect(gateway.loadCount, 3);
    expect(find.textContaining("当前占用 512 MB"), findsOneWidget);
    expect(find.text("当前占用 64 MB"), findsOneWidget);
    expect(find.text("图库数据整理完成"), findsOneWidget);

    oldLoad.complete(oldStatus);
    await tester.pump();
    expect(find.textContaining("当前占用 512 MB"), findsOneWidget);
    expect(find.text("当前占用 64 MB"), findsOneWidget);
    expect(find.textContaining("可回收 512 MB"), findsNothing);
    expect(find.text("图库数据整理完成"), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets("old catalog poll cannot hide a newly started reclamation", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(
      _status(
        catalogReclamation: _catalogReclamation(
          operationId: "old-operation",
          phase: CatalogReclamationPhase.reclaiming,
        ),
      ),
    );
    addTearDown(gateway.dispose);
    var roots = <String>{"root-a"};
    late StateSetter updateRoots;
    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: MaterialApp(
          home: Scaffold(
            body: SingleChildScrollView(
              child: StatefulBuilder(
                builder: (context, setState) {
                  updateRoots = setState;
                  return StorageSettingsSection(
                    hasLibraryRoots: roots.isNotEmpty,
                    libraryRootIds: roots,
                  );
                },
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    final oldPoll = Completer<CatalogReclamationModel>();
    gateway.pendingCatalogReclamation = oldPoll.future;
    await tester.pump(const Duration(milliseconds: 500));
    gateway.status = _status(
      catalogReclamation: _catalogReclamation(
        operationId: "old-operation",
        phase: CatalogReclamationPhase.cancelled,
      ),
    );
    updateRoots(() => roots = {});
    await tester.pump();
    await tester.pump();
    gateway.startedCatalogReclamation = _catalogReclamation(
      operationId: "new-operation",
      phase: CatalogReclamationPhase.queued,
    );
    final row = find.byKey(const Key("catalog-reclamation-setting"));
    await tester.ensureVisible(row);
    await tester.pump();
    await tester.tap(find.descendant(of: row, matching: find.text("重试")));
    await tester.pump();
    expect(find.text("图库数据等待整理"), findsOneWidget);

    oldPoll.complete(
      _catalogReclamation(
        operationId: "old-operation",
        phase: CatalogReclamationPhase.completed,
      ),
    );
    await tester.pump();
    expect(find.text("图库数据等待整理"), findsOneWidget);
    await tester.tap(find.descendant(of: row, matching: find.text("取消")));
    await tester.pump();
    expect(gateway.lastCatalogReclamationCancellation, "new-operation");
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    "settings choices preserve value layout keyboard semantics and disabled state",
    (tester) async {
      final semanticsHandle = tester.ensureSemantics();
      int? selected;
      try {
        await tester.pumpWidget(
          MaterialApp(
            theme: buildAmeTheme(),
            home: Scaffold(
              body: Column(
                children: [
                  SettingsChoice<int>(
                    key: const Key("enabled-settings-choice"),
                    value: 1,
                    entries: const [
                      SettingsChoiceEntry(value: 1, label: "当前"),
                      SettingsChoiceEntry(value: 2, label: "备用"),
                    ],
                    onSelected: (value) => selected = value,
                  ),
                  SettingsChoice<int>(
                    key: const Key("disabled-settings-choice"),
                    value: 3,
                    enabled: false,
                    width: 144,
                    entries: const [
                      SettingsChoiceEntry(value: 3, label: "已禁用"),
                      SettingsChoiceEntry(value: 4, label: "不可选择"),
                    ],
                    onSelected: (_) {},
                  ),
                ],
              ),
            ),
          ),
        );

        final enabledChoice = find.byKey(const Key("enabled-settings-choice"));
        final enabledButton = find.descendant(
          of: enabledChoice,
          matching: find.byType(OutlinedButton),
        );
        expect(tester.getSize(enabledButton), const Size(176, 56));
        expect(
          tester.getSemantics(enabledButton),
          matchesSemantics(
            label: "当前",
            isButton: true,
            hasEnabledState: true,
            isEnabled: true,
            hasTapAction: true,
            hasFocusAction: true,
            isFocusable: true,
          ),
        );

        await tester.sendKeyEvent(LogicalKeyboardKey.tab);
        await tester.pump();
        await tester.sendKeyEvent(LogicalKeyboardKey.enter);
        await tester.pump();
        expect(
          find.descendant(
            of: enabledChoice,
            matching: find.byIcon(Symbols.arrow_drop_up_rounded),
          ),
          findsOneWidget,
        );
        expect(find.byType(MenuAnchor), findsNothing);
        await tester.pumpAndSettle();
        expect(
          tester
              .getSemantics(
                find.byKey(const ValueKey("settings-choice-option-1")),
              )
              .flagsCollection
              .isChecked,
          CheckedState.isTrue,
        );
        final option = find.byKey(const ValueKey("settings-choice-option-2"));
        final route = ModalRoute.of(tester.element(option));
        expect(tester.getSize(option).width, 176);
        expect(route?.transitionDuration, AmeMenuTransition.duration);
        expect(route?.reverseTransitionDuration, AmeMenuTransition.duration);
        await tester.tap(option);
        await tester.pumpAndSettle();
        expect(selected, 2);

        final disabledChoice = find.byKey(
          const Key("disabled-settings-choice"),
        );
        final disabledButton = find.descendant(
          of: disabledChoice,
          matching: find.byType(OutlinedButton),
        );
        expect(tester.getSize(disabledButton), const Size(144, 56));
        expect(
          tester.getSemantics(disabledButton),
          matchesSemantics(
            label: "已禁用",
            isButton: true,
            hasEnabledState: true,
            isEnabled: false,
          ),
        );
        await tester.tap(disabledButton, warnIfMissed: false);
        await tester.pumpAndSettle();
        expect(find.text("不可选择").hitTestable(), findsNothing);
      } finally {
        semanticsHandle.dispose();
      }
    },
  );

  testWidgets("changes the persisted preview loading speed", (tester) async {
    final gateway = _FakeStorageSettingsGateway(_status());
    final preferenceStore = _RecordingAmePreferenceStore();
    addTearDown(gateway.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          amePreferenceStoreProvider.overrideWithValue(preferenceStore),
          storageSettingsGatewayProvider.overrideWithValue(gateway),
        ],
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: const Scaffold(body: AmeSettingsPage(hasLibraryRoots: true)),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final speedMenu = find.descendant(
      of: find.byKey(const Key("preview-loading-speed-setting")),
      matching: find.byType(SettingsChoice<PreviewLoadingSpeed>),
    );
    await tester.ensureVisible(speedMenu);
    await tester.pumpAndSettle();
    final speedSetting = find.byKey(const Key("preview-loading-speed-setting"));
    final speedButton = find.descendant(
      of: speedSetting,
      matching: find.byType(OutlinedButton),
    );
    await _openPopupMenu(tester, speedButton);
    expect(
      find.descendant(
        of: speedSetting,
        matching: find.byIcon(Symbols.arrow_drop_up_rounded),
      ),
      findsOneWidget,
    );
    expect(find.byType(MenuAnchor), findsNothing);
    await _dismissPopupMenu(tester);
    expect(find.text("大").hitTestable(), findsNothing);
    expect(
      find.descendant(
        of: speedSetting,
        matching: find.byIcon(Symbols.arrow_drop_down_rounded),
      ),
      findsOneWidget,
    );

    await _openPopupMenu(tester, speedButton);
    final largeOption = _popupMenuItemTarget("大");
    expect(
      DefaultTextStyle.of(tester.element(find.text("大"))).style.fontWeight,
      ameFontWeightMedium,
    );
    await _selectPopupMenuItem(tester, largeOption);

    expect(
      preferenceStore.saved?.previewLoadingSpeed,
      PreviewLoadingSpeed.large,
    );
  });

  testWidgets("publishes a persistent notification when settings cannot save", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(_status());
    addTearDown(gateway.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          amePreferenceStoreProvider.overrideWithValue(
            const _FailingAmePreferenceStore(),
          ),
          storageSettingsGatewayProvider.overrideWithValue(gateway),
        ],
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: const Scaffold(body: AmeSettingsPage(hasLibraryRoots: true)),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final themeMenu = find.byType(SettingsChoice<AmeThemePreference>);
    await _openPopupMenu(
      tester,
      find.descendant(of: themeMenu, matching: find.byType(OutlinedButton)),
    );
    await _selectPopupMenuItem(tester, _popupMenuItemTarget("浅色"));

    final container = ProviderScope.containerOf(
      tester.element(find.byType(AmeSettingsPage)),
    );
    final notification = container
        .read(ameNotificationControllerProvider)
        .current;
    expect(notification?.title, "无法保存设置");
    expect(notification?.detail, contains("controlled settings failure"));
    expect(notification?.isPersistent, isTrue);
  });

  testWidgets("saves the preview budget without a dialog save action", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(_status());
    addTearDown(gateway.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: const Scaffold(body: AmeSettingsPage(hasLibraryRoots: false)),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final budgetMenu = find.descendant(
      of: find.byKey(const Key("preview-budget-setting")),
      matching: find.byType(SettingsChoice<BigInt>),
    );
    await tester.ensureVisible(budgetMenu);
    await tester.pumpAndSettle();
    final budgetSetting = find.byKey(const Key("preview-budget-setting"));
    final budgetButton = find.descendant(
      of: budgetSetting,
      matching: find.byType(OutlinedButton),
    );
    await _openPopupMenu(tester, budgetButton);
    expect(_popupMenuItemTarget("8 GB").hitTestable(), findsOneWidget);
    await _dismissPopupMenu(tester);
    expect(find.text("8 GB").hitTestable(), findsNothing);

    await _openPopupMenu(tester, budgetButton);
    await _selectPopupMenuItem(tester, _popupMenuItemTarget("8 GB"));

    expect(gateway.lastPreviewBudgetBytes, _gibibytes(8));
    expect(
      find.byKey(const Key("storage-settings-restart-notice")),
      findsOneWidget,
    );
    expect(find.text("现有文件不会被移动或删除"), findsOneWidget);
    expect(find.byKey(const Key("storage-settings-save-button")), findsNothing);
  });

  testWidgets("confirms and reports foreground preview cleanup", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(_status());
    addTearDown(gateway.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: const Scaffold(body: AmeSettingsPage(hasLibraryRoots: true)),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final cleanupRow = find.byKey(const Key("preview-cleanup-setting"));
    final cleanupButton = find.descendant(
      of: cleanupRow,
      matching: find.text("清理"),
    );
    await Scrollable.ensureVisible(
      tester.element(cleanupButton),
      alignment: 0.5,
      duration: Duration.zero,
    );
    await tester.pumpAndSettle();
    await tester.pumpAndSettle();
    await tester.tap(cleanupButton.hitTestable());
    await tester.pumpAndSettle();

    expect(find.byType(AlertDialog), findsOneWidget);
    expect(find.text("清理缩略图？"), findsOneWidget);
    expect(find.textContaining("不会删除或修改原图片"), findsOneWidget);

    await tester.tap(find.text("开始清理"));
    await tester.pump();
    final operationId = gateway.lastCleanupOperationId!;
    gateway.cleanupController.add(
      PreviewCleanupUpdate(
        operationId: operationId,
        phase: PreviewCleanupPhase.started,
        processedFiles: BigInt.zero,
        totalFiles: BigInt.from(2),
        removedFiles: BigInt.zero,
        removedBytes: BigInt.zero,
        issueCount: BigInt.zero,
      ),
    );
    await tester.pump();
    gateway.cleanupController.add(
      PreviewCleanupUpdate(
        operationId: operationId,
        phase: PreviewCleanupPhase.running,
        processedFiles: BigInt.one,
        totalFiles: BigInt.from(2),
        removedFiles: BigInt.one,
        removedBytes: BigInt.from(1024),
        issueCount: BigInt.zero,
      ),
    );
    await tester.pump();

    expect(find.text("正在清理缩略图"), findsOneWidget);
    expect(find.textContaining("已处理 1 / 2 个文件"), findsOneWidget);
    expect(
      find.descendant(of: cleanupRow, matching: find.text("取消")),
      findsOneWidget,
    );

    gateway.cleanupController.add(
      PreviewCleanupUpdate(
        operationId: operationId,
        phase: PreviewCleanupPhase.completed,
        processedFiles: BigInt.from(2),
        totalFiles: BigInt.from(2),
        removedFiles: BigInt.from(2),
        removedBytes: BigInt.from(2048),
        issueCount: BigInt.zero,
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text("缩略图清理完成"), findsOneWidget);
    expect(find.textContaining("已移除 2 个文件"), findsOneWidget);
    expect(find.text("清理"), findsOneWidget);
  });

  testWidgets("isolates and clears a preview cleanup transport error", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(_status());
    addTearDown(gateway.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: const Scaffold(body: AmeSettingsPage(hasLibraryRoots: true)),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final cleanupRow = find.byKey(const Key("preview-cleanup-setting"));
    await tester.ensureVisible(cleanupRow);
    await tester.pump();
    await tester.tap(
      find.descendant(of: cleanupRow, matching: find.text("清理")).hitTestable(),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text("开始清理"));
    await tester.pump();

    final operationId = gateway.lastCleanupOperationId!;
    gateway.cleanupController.addError(
      StateError("controlled preview cleanup failure"),
    );
    await tester.pump();

    expect(find.byKey(const Key("preview-cleanup-error")), findsOneWidget);
    expect(find.text("缩略图清理状态异常"), findsOneWidget);
    expect(find.byKey(const Key("storage-settings-error")), findsNothing);
    expect(find.byKey(const Key("catalog-reclamation-error")), findsNothing);

    gateway.cleanupController.add(
      PreviewCleanupUpdate(
        operationId: operationId,
        phase: PreviewCleanupPhase.running,
        processedFiles: BigInt.one,
        totalFiles: BigInt.from(2),
        removedFiles: BigInt.one,
        removedBytes: BigInt.from(1024),
        issueCount: BigInt.zero,
      ),
    );
    await tester.pump();

    expect(find.byKey(const Key("preview-cleanup-error")), findsNothing);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets("cleans only an owned retired preview root after confirmation", (
    tester,
  ) async {
    final retiredRoot = RetiredPreviewRootModel(
      previewRoot: "D:\\OldAmePreviews\\ame-jpeg-thumbnail-v2-orientation",
      displayPath: "D:\\OldAmePreviews\\ame-jpeg-thumbnail-v2-orientation",
    );
    final gateway = _FakeStorageSettingsGateway(
      _status(retiredPreviewRoots: [retiredRoot]),
    );
    addTearDown(gateway.dispose);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: MaterialApp(
          theme: buildAmeTheme(),
          home: const Scaffold(body: AmeSettingsPage(hasLibraryRoots: true)),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final retiredRow = find.byKey(
      ValueKey("retired-preview-root-${retiredRoot.previewRoot}"),
    );
    final cleanupButton = find.descendant(
      of: retiredRow,
      matching: find.text("清理旧目录"),
    );
    await Scrollable.ensureVisible(
      tester.element(cleanupButton),
      alignment: 0.5,
      duration: Duration.zero,
    );
    await tester.pumpAndSettle();
    await tester.tap(cleanupButton.hitTestable());
    await tester.pumpAndSettle();

    expect(find.text("清理旧缩略图目录？"), findsOneWidget);
    expect(find.textContaining("不会删除原图片或目录中的其他文件"), findsOneWidget);
    await tester.tap(find.text("开始清理"));
    await tester.pump();

    expect(gateway.lastRetiredPreviewRoot, retiredRoot.previewRoot);
    final operationId = gateway.lastCleanupOperationId!;
    gateway.cleanupController.add(
      PreviewCleanupUpdate(
        operationId: operationId,
        phase: PreviewCleanupPhase.completed,
        processedFiles: BigInt.one,
        totalFiles: BigInt.one,
        removedFiles: BigInt.one,
        removedBytes: BigInt.from(1024),
        issueCount: BigInt.zero,
      ),
    );
    await tester.pumpAndSettle();
  });
}

StorageStatusModel _status({
  bool requiresRestart = false,
  List<RetiredPreviewRootModel> retiredPreviewRoots = const [],
  BigInt? catalogUsedBytes,
  BigInt? catalogLiveBytes,
  BigInt? catalogReclaimableBytes,
  BigInt? previewUsedBytes,
  CatalogReclamationModel? catalogReclamation,
}) {
  final usedBytes = catalogUsedBytes ?? BigInt.from(4096);
  final liveBytes = catalogLiveBytes ?? usedBytes;
  final reclaimableBytes = catalogReclaimableBytes ?? BigInt.zero;
  return StorageStatusModel(
    settingsPath: "C:\\AmeConfig\\settings.sqlite3",
    activeCatalogPath: "C:\\AmeData\\catalog\\ame.sqlite3",
    activePreviewRoot: "C:\\AmeCache\\ame-jpeg-thumbnail-v1",
    configuredCatalogPath: r"\\?\C:\AmeData\catalog\ame.sqlite3",
    configuredPreviewRoot: r"\\?\C:\AmeCache\ame-jpeg-thumbnail-v1",
    configuredCatalogDisplayPath: "C:\\AmeData\\catalog\\ame.sqlite3",
    configuredPreviewDisplayPath: "C:\\AmeCache\\ame-jpeg-thumbnail-v1",
    previewBudgetBytes: _gibibytes(4),
    previewUsedBytes: previewUsedBytes ?? BigInt.from(128 * 1024 * 1024),
    catalogUsedBytes: usedBytes,
    catalogLiveBytes: liveBytes,
    catalogReclaimableBytes: reclaimableBytes,
    catalogReclamation:
        catalogReclamation ??
        _catalogReclamation(
          liveBytes: liveBytes,
          reclaimableBytes: reclaimableBytes,
        ),
    requiresRestart: requiresRestart,
    retiredPreviewRoots: retiredPreviewRoots,
  );
}

CatalogReclamationModel _catalogReclamation({
  CatalogReclamationPhase phase = CatalogReclamationPhase.idle,
  String? operationId,
  BigInt? liveBytes,
  BigInt? reclaimableBytes,
}) {
  return CatalogReclamationModel(
    operationId: operationId,
    phase: phase,
    catalogFileBytes:
        (liveBytes ?? BigInt.zero) + (reclaimableBytes ?? BigInt.zero),
    liveBytes: liveBytes ?? BigInt.zero,
    reclaimableBytes: reclaimableBytes ?? BigInt.zero,
    reclaimedBytes: BigInt.zero,
    requiredTemporaryBytes: null,
    availableTemporaryBytes: null,
    errorCode: null,
    errorMessage: null,
  );
}

BigInt _gibibytes(int count) {
  return BigInt.from(count) * BigInt.from(1024 * 1024 * 1024);
}

Finder _popupMenuItemTarget(String label) {
  final item = find.ancestor(
    of: find.text(label).last,
    matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
  );
  return find.descendant(of: item, matching: find.byType(InkWell));
}

Future<void> _openPopupMenu(WidgetTester tester, Finder button) async {
  await tester.tap(button.hitTestable());
  await tester.pump();
  await tester.pump(AmeMenuTransition.duration);
  await tester.pump();
}

Future<void> _dismissPopupMenu(WidgetTester tester) async {
  await tester.sendKeyEvent(LogicalKeyboardKey.escape);
  await tester.pump();
  await tester.pump(AmeMenuTransition.duration);
  await tester.pump();
}

Future<void> _selectPopupMenuItem(WidgetTester tester, Finder target) async {
  await tester.tap(target.hitTestable());
  await tester.pump();
  await tester.pump(AmeMenuTransition.duration);
  await tester.pump();
}

class _FakeStorageSettingsGateway implements StorageSettingsGateway {
  _FakeStorageSettingsGateway(this.status);

  StorageStatusModel status;
  BigInt? lastPreviewBudgetBytes;
  String? lastCleanupOperationId;
  String? lastRetiredPreviewRoot;
  String? lastCatalogReclamationCancellation;
  CatalogReclamationModel? startedCatalogReclamation;
  Future<CatalogReclamationModel>? pendingCatalogReclamation;
  Future<StorageStatusModel>? pendingStorageLoad;
  Object? loadError;
  Object? catalogReclamationLoadError;
  int loadCount = 0;
  final cleanupController = StreamController<PreviewCleanupUpdate>.broadcast();

  @override
  Future<StorageStatusModel> load() async {
    loadCount += 1;
    final pending = pendingStorageLoad;
    pendingStorageLoad = null;
    if (pending != null) {
      return pending;
    }
    final error = loadError;
    if (error != null) {
      throw error;
    }
    return status;
  }

  @override
  Stream<PreviewCleanupUpdate> clearPreviews({required String operationId}) {
    lastCleanupOperationId = operationId;
    return cleanupController.stream;
  }

  @override
  Stream<PreviewCleanupUpdate> clearRetiredPreviews({
    required String previewRoot,
    required String operationId,
  }) {
    lastRetiredPreviewRoot = previewRoot;
    lastCleanupOperationId = operationId;
    return cleanupController.stream;
  }

  @override
  Future<bool> cancelPreviewCleanup({required String operationId}) async {
    return operationId == lastCleanupOperationId;
  }

  @override
  Future<CatalogReclamationModel> startCatalogReclamation({
    required String operationId,
  }) async {
    return startedCatalogReclamation ?? status.catalogReclamation;
  }

  @override
  Future<CatalogReclamationModel> loadCatalogReclamation() async {
    final error = catalogReclamationLoadError;
    if (error != null) {
      throw error;
    }
    final pending = pendingCatalogReclamation;
    if (pending != null) {
      return pending;
    }
    return status.catalogReclamation;
  }

  @override
  Future<bool> cancelCatalogReclamation({required String operationId}) async {
    lastCatalogReclamationCancellation = operationId;
    return (startedCatalogReclamation ?? status.catalogReclamation)
            .operationId ==
        operationId;
  }

  Future<void> dispose() => cleanupController.close();

  @override
  Future<StorageStatusModel> update({
    String? catalogDirectory,
    String? previewCacheDirectory,
    required BigInt previewBudgetBytes,
  }) async {
    lastPreviewBudgetBytes = previewBudgetBytes;
    status = StorageStatusModel(
      settingsPath: status.settingsPath,
      activeCatalogPath: status.activeCatalogPath,
      activePreviewRoot: status.activePreviewRoot,
      configuredCatalogPath: status.configuredCatalogPath,
      configuredPreviewRoot: status.configuredPreviewRoot,
      configuredCatalogDisplayPath: status.configuredCatalogDisplayPath,
      configuredPreviewDisplayPath: status.configuredPreviewDisplayPath,
      previewBudgetBytes: previewBudgetBytes,
      previewUsedBytes: status.previewUsedBytes,
      catalogUsedBytes: status.catalogUsedBytes,
      catalogLiveBytes: status.catalogLiveBytes,
      catalogReclaimableBytes: status.catalogReclaimableBytes,
      catalogReclamation: status.catalogReclamation,
      requiresRestart: true,
      retiredPreviewRoots: status.retiredPreviewRoots,
    );
    return status;
  }
}

class _RecordingAmePreferenceStore implements AmePreferenceStore {
  AmePreferences? saved;

  @override
  Future<AmePreferences> loadAmePreferences() async {
    return saved ?? const AmePreferences();
  }

  @override
  Future<void> saveAmePreferences(AmePreferences preferences) async {
    saved = preferences;
  }
}

class _FailingAmePreferenceStore implements AmePreferenceStore {
  const _FailingAmePreferenceStore();

  @override
  Future<AmePreferences> loadAmePreferences() async => const AmePreferences();

  @override
  Future<void> saveAmePreferences(AmePreferences preferences) {
    throw StateError("controlled settings failure");
  }
}
