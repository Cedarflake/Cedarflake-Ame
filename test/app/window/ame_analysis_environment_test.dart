import "dart:async";

import "package:cedarflake_ame/app/window/ame_analysis_environment.dart";
import "package:flutter/services.dart";
import "package:flutter/widgets.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("a late initial snapshot cannot release a suspended system", (
    tester,
  ) async {
    final fixture = _Fixture(tester);
    final opening = fixture.owner.start();
    expect(fixture.states, [true]);
    await fixture.event(1, true);
    fixture.initial.complete({"revision": 0, "suspended": false});
    await opening;
    fixture.owner.didChangeAppLifecycleState(AppLifecycleState.resumed);
    expect(fixture.states, [true]);
    await fixture.event(2, false);
    expect(fixture.states, [true, false]);
  });

  testWidgets("focus and minimized states do not suspend background analysis", (
    tester,
  ) async {
    final fixture = _Fixture(tester);
    await fixture.start();
    fixture.owner.didChangeAppLifecycleState(AppLifecycleState.resumed);
    fixture.states.clear();
    fixture.owner.didChangeAppLifecycleState(AppLifecycleState.inactive);
    fixture.owner.didChangeAppLifecycleState(AppLifecycleState.hidden);
    fixture.owner.didChangeAppLifecycleState(AppLifecycleState.resumed);
    expect(fixture.states, isEmpty);
    fixture.owner.didChangeAppLifecycleState(AppLifecycleState.paused);
    await fixture.event(1, true);
    await fixture.event(2, false);
    expect(fixture.states, [true]);
    fixture.owner.didChangeAppLifecycleState(AppLifecycleState.resumed);
    expect(fixture.states, [true, false]);
  });

  testWidgets(
    "wake without suspend fences old work and ignores duplicate snapshots",
    (tester) async {
      final fixture = _Fixture(tester);
      await fixture.start();
      fixture.owner.didChangeAppLifecycleState(AppLifecycleState.resumed);
      fixture.states.clear();
      await fixture.event(1, false);
      await fixture.event(1, false);
      await fixture.event(0, true);
      expect(fixture.states, [true, false]);
      expect(fixture.errors, isEmpty);
    },
  );

  testWidgets("disposal removes observers and rejects a late snapshot", (
    tester,
  ) async {
    final fixture = _Fixture(tester);
    final opening = fixture.owner.start();
    fixture.owner.didHaveMemoryPressure();
    expect(fixture.pressures, 1);
    fixture.owner.dispose();
    fixture.initial.complete({"revision": 0, "suspended": false});
    await opening;
    fixture.owner.didHaveMemoryPressure();
    fixture.owner.didChangeAppLifecycleState(AppLifecycleState.resumed);
    expect(fixture.states, [true]);
    expect(fixture.pressures, 1);
    expect(fixture.errors, isEmpty);
  });

  testWidgets("registration failure inhibits analysis and reports its error", (
    tester,
  ) async {
    final fixture = _Fixture(tester);
    final opening = fixture.owner.start();
    fixture.initial.completeError(
      PlatformException(code: "system_power_unavailable"),
    );
    await opening;
    expect(fixture.states, [true]);
    expect(fixture.errors.single, isA<PlatformException>());
    await fixture.event(1, false);
    expect(fixture.states, [true]);
  });

  testWidgets("inconsistent native state cannot resume analysis", (
    tester,
  ) async {
    final fixture = _Fixture(tester);
    await fixture.start();
    fixture.owner.didChangeAppLifecycleState(AppLifecycleState.resumed);
    fixture.states.clear();
    await fixture.event(0, true);
    expect(fixture.states, [true]);
    expect(fixture.errors.single, isA<FormatException>());
  });
}

class _Fixture {
  _Fixture(this.tester) {
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (
      call,
    ) {
      expect(call.method, "getState");
      return initial.future;
    });
    owner = AmeAnalysisEnvironment(
      onSuspendedChanged: states.add,
      onMemoryPressure: () => pressures++,
      onError: (error, _) => errors.add(error),
      powerChannel: channel,
    );
    addTearDown(() {
      owner.dispose();
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        channel,
        null,
      );
    });
  }

  final WidgetTester tester;
  static const channel = MethodChannel("ame/test_power");
  final initial = Completer<Object?>();
  final states = <bool>[];
  final errors = <Object>[];
  var pressures = 0;
  late final AmeAnalysisEnvironment owner;

  Future<void> start() async {
    initial.complete({"revision": 0, "suspended": false});
    await owner.start();
  }

  Future<void> event(int revision, bool suspended) async {
    final reply = Completer<void>();
    await tester.binding.defaultBinaryMessenger.handlePlatformMessage(
      channel.name,
      channel.codec.encodeMethodCall(
        MethodCall("stateChanged", {
          "revision": revision,
          "suspended": suspended,
        }),
      ),
      (_) => reply.complete(),
    );
    await reply.future;
  }
}
