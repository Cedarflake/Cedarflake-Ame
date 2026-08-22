import "dart:async";

import "package:cedarflake_ame/app/window/ame_shutdown_coordinator.dart";
import "package:cedarflake_ame/app/window/ame_window_placement.dart";
import "package:cedarflake_ame/app/window/window_manager_actions.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";
import "package:window_manager/window_manager.dart";

void main() {
  test(
    "restores normal bounds and maximized state before first show",
    () async {
      final window = _FakeWindowBootstrapActions();
      const placement = AmeWindowPlacement(
        left: 120,
        top: 80,
        width: 1440,
        height: 900,
        isMaximized: true,
      );

      final normalPlacement = await restoreAmeWindowBeforeShow(
        window: window,
        restoredPlacement: placement,
        shouldMaximize: true,
      );

      expect(window.calls, [
        "ready",
        "position",
        "get-position",
        "get-size",
        "maximize",
        "show",
        "focus",
      ]);
      expect(window.options?.size, const Size(1440, 900));
      expect(window.options?.minimumSize, const Size(800, 560));
      expect(window.position, const Offset(120, 80));
      expect(normalPlacement.bounds, const Rect.fromLTWH(120, 80, 1440, 900));
      expect(normalPlacement.isMaximized, isFalse);
    },
  );

  test(
    "close destroys the window after the bounded shutdown timeout",
    () async {
      final coordinator = AmeShutdownCoordinator();
      final shutdownBlocker = Completer<void>();
      coordinator.register(() => shutdownBlocker.future);
      var destroyCount = 0;
      final actions = WindowManagerActions(
        _MemoryWindowPreferenceStore(),
        shutdownCoordinator: coordinator,
        maximumShutdownDuration: const Duration(milliseconds: 1),
        hideWindow: () async {},
        destroyWindow: () async => destroyCount += 1,
      );

      await Future.wait([actions.close(), actions.close()]);

      expect(destroyCount, 1);
      shutdownBlocker.complete();
    },
  );

  test("close hides the window before waiting for shutdown", () async {
    final coordinator = AmeShutdownCoordinator();
    final shutdownBlocker = Completer<void>();
    coordinator.register(() => shutdownBlocker.future);
    final calls = <String>[];
    final actions = WindowManagerActions(
      _MemoryWindowPreferenceStore(),
      shutdownCoordinator: coordinator,
      maximumShutdownDuration: const Duration(seconds: 1),
      hideWindow: () async => calls.add("hide"),
      destroyWindow: () async => calls.add("destroy"),
    );

    final close = actions.close();
    await Future<void>.delayed(Duration.zero);

    expect(calls, ["hide"]);
    shutdownBlocker.complete();
    await close;
    expect(calls, ["hide", "destroy"]);
  });
}

class _MemoryWindowPreferenceStore implements AmeWindowPreferenceStore {
  @override
  Future<AmeWindowPlacement?> loadWindowPlacement() async => null;

  @override
  Future<void> saveWindowPlacement(AmeWindowPlacement placement) async {}
}

class _FakeWindowBootstrapActions implements AmeWindowBootstrapActions {
  final List<String> calls = [];
  WindowOptions? options;
  Offset position = const Offset(10, 10);
  Size size = const Size(1280, 720);

  @override
  Future<void> waitUntilReadyToShow(WindowOptions options) async {
    calls.add("ready");
    this.options = options;
    size = options.size ?? size;
  }

  @override
  Future<void> setPosition(Offset position) async {
    calls.add("position");
    this.position = position;
  }

  @override
  Future<Offset> getPosition() async {
    calls.add("get-position");
    return position;
  }

  @override
  Future<Size> getSize() async {
    calls.add("get-size");
    return size;
  }

  @override
  Future<void> maximize() async {
    calls.add("maximize");
  }

  @override
  Future<void> show() async {
    calls.add("show");
  }

  @override
  Future<void> focus() async {
    calls.add("focus");
  }
}
