import "package:cedarflake_ame/app/presentation/ame_loading_indicator.dart";
import "package:cedarflake_ame/app/presentation/ame_progress.dart";
import "package:flutter_test/flutter_test.dart";
import "package:material_ui/material_ui.dart";

void main() {
  testWidgets("quiet, reduced-motion and hidden feedback stop animation work", (
    tester,
  ) async {
    Future<void> show({
      bool loading = true,
      bool reduced = false,
      bool active = true,
    }) => tester.pumpWidget(
      MaterialApp(
        home: MediaQuery(
          data: MediaQueryData(disableAnimations: reduced),
          child: TickerMode(
            enabled: active,
            child: Center(child: AmeLoadingProgress(active: loading)),
          ),
        ),
      ),
    );
    await show();
    await tester.pump(const Duration(milliseconds: 200));
    expect(tester.binding.transientCallbackCount, 0);
    await tester.pump(const Duration(milliseconds: 50));
    await tester.pump(const Duration(seconds: 1));
    expect(
      tester.getSize(find.byType(AmeLoadingIndicator)),
      const Size(48, 48),
    );
    expect(tester.binding.transientCallbackCount, greaterThan(0));
    await show(reduced: true);
    await tester.pump();
    expect(tester.binding.transientCallbackCount, 0);
    await show(active: false);
    await tester.pump();
    expect(tester.binding.transientCallbackCount, 0);
    await show();
    await tester.pump();
    expect(tester.binding.transientCallbackCount, greaterThan(0));
    await show(loading: false);
    await tester.pump(const Duration(seconds: 1));
    expect(tester.binding.transientCallbackCount, 0);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });
}
