import "package:cedarflake_ame/app/ame_window_frame.dart";
import "package:cedarflake_ame/app/window/ame_window_actions.dart";
import "package:flutter/foundation.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_symbols_icons/symbols.dart";
import "package:window_manager/window_manager.dart";

void main() {
  testWidgets("draws Ame chrome with Material caption actions", (tester) async {
    final actions = _FakeWindowActions();
    addTearDown(actions.dispose);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [ameWindowActionsProvider.overrideWithValue(actions)],
        child: const MaterialApp(home: AmeWindowFrame(child: Text("内容"))),
      ),
    );

    expect(find.text("Cedarflake Ame"), findsOneWidget);
    expect(find.byType(DragToMoveArea), findsOneWidget);
    expect(find.byKey(const Key("window-minimize")), findsOneWidget);
    expect(find.byKey(const Key("window-maximize")), findsOneWidget);
    expect(find.byKey(const Key("window-close")), findsOneWidget);
    expect(find.byIcon(Symbols.horizontal_rule_rounded), findsOneWidget);
    expect(find.byIcon(Symbols.crop_square_rounded), findsOneWidget);
    expect(find.byIcon(Symbols.close_rounded), findsOneWidget);
    expect(
      tester.widget<Icon>(find.byIcon(Symbols.horizontal_rule_rounded)).size,
      22,
    );
    expect(
      tester.widget<Icon>(find.byIcon(Symbols.crop_square_rounded)).size,
      18,
    );
    expect(tester.widget<Icon>(find.byIcon(Symbols.close_rounded)).size, 24);
    expect(
      tester.getSize(find.byKey(const Key("window-minimize"))),
      const Size.square(40),
    );

    await tester.tap(find.byKey(const Key("window-minimize")));
    await tester.tap(find.byKey(const Key("window-maximize")));
    await tester.tap(find.byKey(const Key("window-close")));
    await tester.pump();

    expect(actions.minimizeCount, 1);
    expect(actions.toggleMaximizeCount, 1);
    expect(actions.closeCount, 1);
    expect(find.byIcon(Symbols.filter_none_rounded), findsOneWidget);
  });
}

class _FakeWindowActions implements AmeWindowActions {
  final ValueNotifier<bool> _isMaximized = ValueNotifier(false);
  int minimizeCount = 0;
  int toggleMaximizeCount = 0;
  int closeCount = 0;

  @override
  ValueListenable<bool> get isMaximized => _isMaximized;

  @override
  Future<void> close() async {
    closeCount++;
  }

  @override
  void dispose() {
    _isMaximized.dispose();
  }

  @override
  Future<void> minimize() async {
    minimizeCount++;
  }

  @override
  Future<void> toggleMaximize() async {
    toggleMaximizeCount++;
    _isMaximized.value = !_isMaximized.value;
  }
}
