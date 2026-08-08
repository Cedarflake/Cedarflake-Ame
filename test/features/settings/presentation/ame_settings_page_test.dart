import "package:cedarflake_ame/features/settings/presentation/ame_settings_page.dart";
import "package:cedarflake_ame/features/storage/application/storage_settings.dart";
import "package:cedarflake_ame/features/storage/domain/storage_models.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("renders settings in the main canvas with plain-language rows", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(_status());

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: const MaterialApp(
          home: Scaffold(body: AmeSettingsPage(hasLibraryRoots: true)),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key("ame-settings-page")), findsOneWidget);
    expect(find.byType(AlertDialog), findsNothing);
    expect(find.text("个性化"), findsOneWidget);
    expect(find.text("浏览"), findsOneWidget);
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
  });

  testWidgets("saves the preview budget without a dialog save action", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(_status());

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: const MaterialApp(
          home: Scaffold(body: AmeSettingsPage(hasLibraryRoots: false)),
        ),
      ),
    );
    await tester.pumpAndSettle();

    final budgetMenu = find.descendant(
      of: find.byKey(const Key("preview-budget-setting")),
      matching: find.byType(DropdownMenu<BigInt>),
    );
    await tester.tap(budgetMenu);
    await tester.pumpAndSettle();
    await tester.tap(find.text("8 GB").last);
    await tester.pumpAndSettle();

    expect(gateway.lastPreviewBudgetBytes, _gibibytes(8));
    expect(
      find.byKey(const Key("storage-settings-restart-notice")),
      findsOneWidget,
    );
    expect(find.text("现有文件不会被移动或删除"), findsOneWidget);
    expect(find.byKey(const Key("storage-settings-save-button")), findsNothing);
  });
}

StorageStatusModel _status({bool requiresRestart = false}) {
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
  );
}

BigInt _gibibytes(int count) {
  return BigInt.from(count) * BigInt.from(1024 * 1024 * 1024);
}

class _FakeStorageSettingsGateway implements StorageSettingsGateway {
  _FakeStorageSettingsGateway(this.status);

  StorageStatusModel status;
  BigInt? lastPreviewBudgetBytes;

  @override
  Future<StorageStatusModel> load() async => status;

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
    );
    return status;
  }
}
