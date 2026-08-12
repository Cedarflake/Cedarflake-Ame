import "dart:async";

import "package:cedarflake_ame/app/presentation/ame_theme.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:cedarflake_ame/features/settings/presentation/ame_settings_page.dart";
import "package:cedarflake_ame/features/storage/application/storage_settings.dart";
import "package:cedarflake_ame/features/storage/domain/storage_models.dart";
import "package:flutter/material.dart";
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
    final menus = tester
        .widgetList<DropdownMenu<dynamic>>(
          find.byWidgetPredicate((widget) => widget is DropdownMenu),
        )
        .toList(growable: false);
    expect(menus, hasLength(5));
    for (final menu in menus) {
      expect((menu.trailingIcon as Icon).icon, Symbols.arrow_drop_down_rounded);
      expect(
        (menu.selectedTrailingIcon as Icon).icon,
        Symbols.arrow_drop_up_rounded,
      );
    }
    expect(find.byIcon(Icons.arrow_drop_down), findsNothing);
    expect(find.byIcon(Icons.arrow_drop_up), findsNothing);
  });

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
      matching: find.byType(DropdownMenu<PreviewLoadingSpeed>),
    );
    await tester.ensureVisible(speedMenu);
    await tester.pumpAndSettle();
    final speedSetting = find.byKey(const Key("preview-loading-speed-setting"));
    await tester.tap(
      find
          .descendant(
            of: speedSetting,
            matching: find.byIcon(Symbols.arrow_drop_down_rounded),
          )
          .hitTestable(),
    );
    await tester.pumpAndSettle();
    expect(
      find.byIcon(Symbols.arrow_drop_up_rounded).hitTestable(),
      findsOneWidget,
    );
    await tester.tap(
      find
          .descendant(
            of: speedSetting,
            matching: find.byIcon(Symbols.arrow_drop_up_rounded),
          )
          .hitTestable(),
    );
    await tester.pumpAndSettle();
    expect(find.text("大").hitTestable(), findsNothing);

    await tester.tap(
      find
          .descendant(
            of: speedSetting,
            matching: find.byIcon(Symbols.arrow_drop_down_rounded),
          )
          .hitTestable(),
    );
    await tester.pumpAndSettle();
    final largeOption = find.text("大").hitTestable();
    expect(
      DefaultTextStyle.of(tester.element(largeOption)).style.fontWeight,
      FontWeight.w400,
    );
    await tester.tap(largeOption);
    await tester.pumpAndSettle();

    expect(
      preferenceStore.saved?.previewLoadingSpeed,
      PreviewLoadingSpeed.large,
    );
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
      matching: find.byType(DropdownMenu<BigInt>),
    );
    await tester.ensureVisible(budgetMenu);
    await tester.pumpAndSettle();
    final budgetSetting = find.byKey(const Key("preview-budget-setting"));
    await tester.tap(
      find
          .descendant(
            of: budgetSetting,
            matching: find.byIcon(Symbols.arrow_drop_down_rounded),
          )
          .hitTestable(),
    );
    await tester.pumpAndSettle();
    expect(find.text("8 GB").hitTestable(), findsOneWidget);
    await tester.tap(
      find
          .descendant(
            of: budgetSetting,
            matching: find.byIcon(Symbols.arrow_drop_up_rounded),
          )
          .hitTestable(),
    );
    await tester.pumpAndSettle();
    expect(find.text("8 GB").hitTestable(), findsNothing);

    await tester.tap(
      find
          .descendant(
            of: budgetSetting,
            matching: find.byIcon(Symbols.arrow_drop_down_rounded),
          )
          .hitTestable(),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text("8 GB").hitTestable());
    await tester.pumpAndSettle();

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
}) {
  return StorageStatusModel(
    settingsPath: "C:\\AmeConfig\\settings.sqlite3",
    activeCatalogPath: "C:\\AmeData\\catalog\\ame.sqlite3",
    activePreviewRoot: "C:\\AmeCache\\ame-jpeg-thumbnail-v1",
    configuredCatalogPath: r"\\?\C:\AmeData\catalog\ame.sqlite3",
    configuredPreviewRoot: r"\\?\C:\AmeCache\ame-jpeg-thumbnail-v1",
    configuredCatalogDisplayPath: "C:\\AmeData\\catalog\\ame.sqlite3",
    configuredPreviewDisplayPath: "C:\\AmeCache\\ame-jpeg-thumbnail-v1",
    previewBudgetBytes: _gibibytes(4),
    previewUsedBytes: BigInt.from(128 * 1024 * 1024),
    catalogUsedBytes: BigInt.from(4096),
    requiresRestart: requiresRestart,
    retiredPreviewRoots: retiredPreviewRoots,
  );
}

BigInt _gibibytes(int count) {
  return BigInt.from(count) * BigInt.from(1024 * 1024 * 1024);
}

class _FakeStorageSettingsGateway implements StorageSettingsGateway {
  _FakeStorageSettingsGateway(this.status);

  StorageStatusModel status;
  BigInt? lastPreviewBudgetBytes;
  String? lastCleanupOperationId;
  String? lastRetiredPreviewRoot;
  final cleanupController = StreamController<PreviewCleanupUpdate>.broadcast();

  @override
  Future<StorageStatusModel> load() async => status;

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
