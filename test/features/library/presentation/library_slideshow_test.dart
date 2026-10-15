import "dart:async";

import "package:cedarflake_ame/features/library/presentation/library_slideshow.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "each slide gets its full dwell after decoding, including slow paging",
    (tester) async {
      final player = LibrarySlideshow();
      addTearDown(player.dispose);
      final pending = Completer<bool>();
      var advances = 0;
      player.show("one", () {
        advances++;
        return pending.future;
      });
      player.toggle();
      await tester.pump(const Duration(seconds: 20));
      expect(advances, 0);
      player.ready("one");
      await tester.pump(const Duration(seconds: 2));
      expect(advances, 0);
      await tester.pump(const Duration(seconds: 1));
      expect(advances, 1);
      await tester.pump(const Duration(seconds: 20));
      expect(advances, 1);
      player.show("two", () async {
        advances++;
        return true;
      });
      pending.complete(true);
      player.ready("one");
      await tester.pump(const Duration(seconds: 5));
      expect(advances, 1);
      player.ready("two");
      await tester.pump(const Duration(seconds: 3));
      expect(advances, 2);
    },
  );

  testWidgets("pause, failed decode and end of collection stop playback", (
    tester,
  ) async {
    final player = LibrarySlideshow();
    addTearDown(player.dispose);
    var advances = 0;
    player.show("one", () async {
      advances++;
      return true;
    });
    player.ready("one");
    player.toggle();
    player.pause();
    await tester.pump(const Duration(seconds: 4));
    expect(advances, 0);
    player.toggle();
    player.failed("one");
    expect(player.isPlaying, isFalse);
    player.show("last", null);
    player.ready("last");
    player.toggle();
    expect(player.isPlaying, isFalse);
  });

  testWidgets("late failure cannot stop a newly started playback", (
    tester,
  ) async {
    final player = LibrarySlideshow();
    addTearDown(player.dispose);
    final old = Completer<bool>();
    player.show("one", () => old.future);
    player.ready("one");
    player.toggle();
    await tester.pump(const Duration(seconds: 3));
    player.pause();
    player.show("two", () async => true);
    player.ready("two");
    player.toggle();
    old.complete(false);
    await tester.pump();
    expect(player.isPlaying, isTrue);
    player.pause();
    await tester.pump(const Duration(seconds: 10));
  });
}
