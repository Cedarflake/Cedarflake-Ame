import "package:cedarflake_ame/features/storage/application/storage_settings.dart";
import "package:cedarflake_ame/features/storage/domain/storage_models.dart";
import "package:cedarflake_ame/features/storage/presentation/storage_settings_dialog.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("locks catalog relocation when the library has sources", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(_status());

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: const MaterialApp(
          home: Scaffold(body: StorageSettingsButton(hasLibraryRoots: true)),
        ),
      ),
    );

    await tester.tap(find.byKey(const Key("storage-settings-button")));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key("storage-settings-dialog")), findsOneWidget);
    expect(find.text("Catalog database"), findsOneWidget);
    expect(find.text("Preview cache"), findsOneWidget);
    expect(find.text("Migration required"), findsOneWidget);
    expect(find.textContaining("Catalog relocation is locked"), findsOneWidget);
  });

  testWidgets("saves a preview budget and reports restart activation", (
    tester,
  ) async {
    final gateway = _FakeStorageSettingsGateway(_status());

    await tester.pumpWidget(
      ProviderScope(
        overrides: [storageSettingsGatewayProvider.overrideWithValue(gateway)],
        child: const MaterialApp(
          home: Scaffold(body: StorageSettingsButton(hasLibraryRoots: false)),
        ),
      ),
    );

    await tester.tap(find.byKey(const Key("storage-settings-button")));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key("preview-budget-field")));
    await tester.pumpAndSettle();
    await tester.tap(find.text("8 GiB").last);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key("storage-settings-save-button")));
    await tester.pumpAndSettle();

    expect(gateway.lastPreviewBudgetBytes, _gibibytes(8));
    expect(find.textContaining("Restart Ame to activate"), findsOneWidget);
    expect(find.textContaining("Existing files are not moved"), findsOneWidget);
  });
}

StorageStatusModel _status({bool requiresRestart = false}) {
  return StorageStatusModel(
    settingsPath: "C:\\AmeConfig\\settings.sqlite3",
    activeCatalogPath: "C:\\AmeData\\catalog\\ame.sqlite3",
    activePreviewRoot: "C:\\AmeCache\\ame-jpeg-thumbnail-v1",
    configuredCatalogPath: "C:\\AmeData\\catalog\\ame.sqlite3",
    configuredPreviewRoot: "C:\\AmeCache\\ame-jpeg-thumbnail-v1",
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
      previewBudgetBytes: previewBudgetBytes,
      previewUsedBytes: status.previewUsedBytes,
      catalogUsedBytes: status.catalogUsedBytes,
      requiresRestart: true,
    );
    return status;
  }
}
