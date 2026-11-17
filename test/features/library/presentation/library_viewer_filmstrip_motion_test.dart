import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_filmstrip_slot.dart";
import "package:material_ui/material_ui.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("filmstrip motion does not repaint unchanged image surfaces", (
    tester,
  ) async {
    final visible = ValueNotifier(true);
    addTearDown(visible.dispose);
    var canvasPaints = 0;
    var stripPaints = 0;
    final canvas = CustomPaint(painter: _CountPaints(() => canvasPaints++));
    final strip = CustomPaint(painter: _CountPaints(() => stripPaints++));
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ValueListenableBuilder(
            valueListenable: visible,
            builder: (context, value, _) => Stack(
              fit: StackFit.expand,
              children: [
                canvas,
                Positioned(
                  left: 0,
                  right: 0,
                  bottom: 0,
                  child: LibraryViewerFilmstripSlot(
                    visible: value,
                    child: strip,
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(canvasPaints, greaterThan(0));
    expect(stripPaints, greaterThan(0));
    for (final expanded in [false, true]) {
      visible.value = expanded;
      await tester.pump();
      final canvasBeforeMotion = canvasPaints;
      final stripBeforeMotion = stripPaints;
      for (var frame = 0; frame < 6; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
      }
      expect(canvasPaints, canvasBeforeMotion);
      expect(stripPaints, stripBeforeMotion);
      await tester.pumpAndSettle();
    }
  });
}

class _CountPaints extends CustomPainter {
  _CountPaints(this.onPaint);

  final VoidCallback onPaint;

  @override
  void paint(Canvas canvas, Size size) {
    onPaint();
    canvas.drawRect(Offset.zero & size, Paint()..color = Colors.blue);
  }

  @override
  bool shouldRepaint(_CountPaints oldDelegate) => false;
}
