import "dart:async";

import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:cedarflake_ame/features/settings/presentation/widgets/analysis_settings_section.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "GPU selection waits for the device change and retains the saved preference across page entry",
    (tester) async {
      final configuration = _Configuration();
      Widget page() => ProviderScope(
        overrides: [
          libraryClassificationConfigurationProvider.overrideWithValue(
            configuration,
          ),
        ],
        child: const MaterialApp(
          home: Scaffold(body: AnalysisSettingsSection()),
        ),
      );
      await tester.pumpWidget(page());
      await tester.pumpAndSettle();
      final toggle = find.byKey(const Key("settings-gpu-acceleration"));
      expect(tester.widget<Switch>(toggle).value, isTrue);
      await tester.tap(toggle);
      await tester.pump();
      expect(configuration.requested, isFalse);
      expect(tester.widget<Switch>(toggle).onChanged, isNull);
      expect(find.text("正在切换分析设备"), findsOneWidget);
      configuration.pending.complete();
      await tester.pumpAndSettle();
      expect(tester.widget<Switch>(toggle).value, isFalse);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpWidget(page());
      await tester.pumpAndSettle();
      expect(tester.widget<Switch>(toggle).value, isFalse);
      configuration.failAfterSave = true;
      await tester.tap(toggle);
      await tester.pumpAndSettle();
      expect(tester.widget<Switch>(toggle).value, isTrue);
      expect(tester.widget<Switch>(toggle).onChanged, isNotNull);
      expect(tester.takeException(), isNull);
    },
  );
}

class _Configuration implements LibraryClassificationConfiguration {
  bool enabled = true;
  bool failAfterSave = false;
  bool? requested;
  final pending = Completer<void>();

  @override
  Future<LibraryClassificationAcceleration> loadAcceleration() async =>
      LibraryClassificationAcceleration(gpuEnabled: enabled);

  @override
  Future<LibraryClassificationAcceleration> changeAcceleration({
    required bool gpuEnabled,
  }) async {
    requested = gpuEnabled;
    await pending.future;
    enabled = gpuEnabled;
    if (failAfterSave) {
      throw StateError("The replacement worker could not open");
    }
    return loadAcceleration();
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
