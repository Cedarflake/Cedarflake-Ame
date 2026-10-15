import "dart:async";

import "package:cedarflake_ame/app/window/ame_window_fullscreen.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "captures normal placement before fullscreen and rejects late bounds",
    () async {
      var native = false;
      var captures = 0;
      late AmeWindowFullscreen fullscreen;
      fullscreen = AmeWindowFullscreen(
        capturePlacement: () async {
          expect(fullscreen.protectsPlacement, isFalse);
          captures++;
        },
        setFullscreen: (value) async {
          expect(fullscreen.protectsPlacement, isTrue);
          native = value;
        },
        readFullscreen: () async => native,
      );
      addTearDown(fullscreen.dispose);
      final generation = fullscreen.placementGeneration;
      await fullscreen.setEnabled(true);
      expect(fullscreen.isFullscreen, isTrue);
      expect(fullscreen.canSavePlacement(generation), isFalse);
      await fullscreen.setEnabled(false);
      expect(fullscreen.isFullscreen, isFalse);
      expect(fullscreen.canSavePlacement(generation), isFalse);
      expect(
        fullscreen.canSavePlacement(fullscreen.placementGeneration),
        isTrue,
      );
      expect(captures, 1);
    },
  );

  test(
    "exit requested during entry finishes in restored window state",
    () async {
      var native = false;
      final entry = Completer<void>();
      final calls = <bool>[];
      final fullscreen = AmeWindowFullscreen(
        capturePlacement: () async {},
        setFullscreen: (value) async {
          calls.add(value);
          if (value) await entry.future;
          native = value;
        },
        readFullscreen: () async => native,
      );
      addTearDown(fullscreen.dispose);
      final enter = fullscreen.setEnabled(true);
      final exit = fullscreen.setEnabled(false);
      entry.complete();
      await Future.wait([enter, exit]);
      expect(calls, [true, false]);
      expect(fullscreen.isFullscreen, isFalse);
      expect(fullscreen.isChanging, isFalse);
    },
  );

  test("a native failure releases controls and allows a later retry", () async {
    var attempts = 0;
    var native = false;
    final fullscreen = AmeWindowFullscreen(
      capturePlacement: () async {},
      setFullscreen: (value) async {
        if (attempts++ == 0) throw StateError("native failure");
        native = value;
      },
      readFullscreen: () async => native,
    );
    addTearDown(fullscreen.dispose);
    await expectLater(fullscreen.setEnabled(true), throwsStateError);
    expect(fullscreen.protectsPlacement, isFalse);
    await fullscreen.setEnabled(true);
    expect(fullscreen.isFullscreen, isTrue);
  });
}
