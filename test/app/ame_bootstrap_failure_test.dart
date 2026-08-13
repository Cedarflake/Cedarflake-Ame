import "package:cedarflake_ame/app/bootstrap/ame_bootstrap_failure.dart";
import "package:cedarflake_ame/app/window/ame_window_actions.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:flutter/foundation.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets("localizes startup failure without exposing raw diagnostics", (
    tester,
  ) async {
    const error = LibraryCatalogFailure(
      code: "catalog_database_error",
      message: "raw sqlite path and lock detail",
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          ameWindowActionsProvider.overrideWithValue(
            const _TestWindowActions(),
          ),
        ],
        child: const AmeBootstrapFailure(
          error: error,
          preferences: AmePreferences(theme: AmeThemePreference.dark),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text("Cedarflake Ame 无法启动"), findsOneWidget);
    expect(find.textContaining("图库数据暂时无法读取"), findsOneWidget);
    expect(find.textContaining("raw sqlite"), findsNothing);
    expect(find.text("复制诊断信息"), findsOneWidget);
    expect(
      tester.widget<MaterialApp>(find.byType(MaterialApp)).themeMode,
      ThemeMode.dark,
    );
  });

  testWidgets("copies raw diagnostics only after an explicit action", (
    tester,
  ) async {
    const diagnostics = "native loader diagnostic";
    String? clipboardText;
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == "Clipboard.setData") {
          clipboardText =
              (call.arguments as Map<Object?, Object?>)["text"] as String?;
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          ameWindowActionsProvider.overrideWithValue(
            const _TestWindowActions(),
          ),
        ],
        child: const AmeBootstrapFailure(
          error: FormatException(diagnostics),
          preferences: AmePreferences(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(clipboardText, isNull);

    await tester.tap(find.text("复制诊断信息"));
    await tester.pumpAndSettle();

    expect(clipboardText, contains(diagnostics));
    expect(find.text("诊断信息已复制"), findsOneWidget);
  });
}

class _TestWindowActions implements AmeWindowActions {
  const _TestWindowActions();

  static final ValueNotifier<bool> _isMaximized = ValueNotifier(false);

  @override
  ValueListenable<bool> get isMaximized => _isMaximized;

  @override
  Future<void> close() async {}

  @override
  void dispose() {}

  @override
  Future<void> minimize() async {}

  @override
  Future<void> toggleMaximize() async {}
}
