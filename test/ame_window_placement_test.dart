import "dart:ui";

import "package:cedarflake_ame/app/window/ame_window_placement.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("keeps a valid placement on its original monitor", () {
    const placement = AmeWindowPlacement(
      left: -1600,
      top: 120,
      width: 1200,
      height: 760,
      isMaximized: true,
    );

    final result = normalizeAmeWindowPlacement(
      placement,
      visibleScreenBounds: const [
        Rect.fromLTWH(0, 0, 1920, 1040),
        Rect.fromLTWH(-1920, 0, 1920, 1040),
      ],
      minimumSize: const Size(800, 560),
    );

    expect(result?.bounds, placement.bounds);
    expect(result?.isMaximized, isTrue);
  });

  test("moves an off-screen placement into the primary work area", () {
    const placement = AmeWindowPlacement(
      left: 4200,
      top: 2000,
      width: 2400,
      height: 1400,
      isMaximized: false,
    );

    final result = normalizeAmeWindowPlacement(
      placement,
      visibleScreenBounds: const [Rect.fromLTWH(0, 0, 1920, 1040)],
      minimumSize: const Size(800, 560),
    );

    expect(result?.bounds, const Rect.fromLTWH(0, 0, 1920, 1040));
    expect(result?.isMaximized, isFalse);
  });

  test("rejects invalid geometry rather than restoring it", () {
    const placement = AmeWindowPlacement(
      left: 0,
      top: 0,
      width: double.nan,
      height: 720,
      isMaximized: false,
    );

    expect(
      normalizeAmeWindowPlacement(
        placement,
        visibleScreenBounds: const [Rect.fromLTWH(0, 0, 1920, 1040)],
        minimumSize: const Size(800, 560),
      ),
      isNull,
    );
  });
}
