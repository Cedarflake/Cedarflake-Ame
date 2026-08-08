import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter/material.dart";
import "package:screen_retriever/screen_retriever.dart";
import "package:window_manager/window_manager.dart";

import "ame_window_actions.dart";
import "ame_window_placement.dart";

const _minimumWindowSize = Size(800, 560);

class WindowManagerActions with WindowListener implements AmeWindowActions {
  WindowManagerActions(
    this._preferenceStore, {
    AmeWindowPlacement? initialNormalPlacement,
  }) : _normalPlacement = initialNormalPlacement?.copyWith(isMaximized: false);

  final AmeWindowPreferenceStore _preferenceStore;
  final ValueNotifier<bool> _isMaximized = ValueNotifier(false);
  AmeWindowPlacement? _normalPlacement;
  Timer? _placementSaveDebounce;
  bool _isClosing = false;

  @override
  ValueListenable<bool> get isMaximized => _isMaximized;

  Future<void> initialize() async {
    windowManager.addListener(this);
    _isMaximized.value = await windowManager.isMaximized();
    if (!_isMaximized.value && !await windowManager.isMinimized()) {
      await _captureNormalPlacement();
    }
  }

  @override
  Future<void> minimize() => windowManager.minimize();

  @override
  Future<void> toggleMaximize() async {
    if (await windowManager.isMaximized()) {
      await windowManager.unmaximize();
      _isMaximized.value = false;
      if (!_isClosing) {
        await _captureNormalPlacement();
      }
      return;
    }
    try {
      await _captureNormalPlacement();
    } on Object {
      // Window actions remain available when non-critical preference I/O fails.
    }
    await windowManager.maximize();
    _isMaximized.value = true;
    if (!_isClosing) {
      await _saveKnownPlacement(isMaximized: true);
    }
  }

  @override
  Future<void> close() {
    _beginClosing();
    return windowManager.close();
  }

  @override
  void onWindowClose() {
    _beginClosing();
  }

  @override
  void onWindowMaximize() {
    if (_isClosing) {
      return;
    }
    _placementSaveDebounce?.cancel();
    _isMaximized.value = true;
    _runBackground(_saveKnownPlacement(isMaximized: true));
  }

  @override
  void onWindowMinimize() {
    _placementSaveDebounce?.cancel();
  }

  @override
  void onWindowMoved() {
    if (_isClosing) {
      return;
    }
    _scheduleNormalPlacementSave();
  }

  @override
  void onWindowResized() {
    if (_isClosing) {
      return;
    }
    _scheduleNormalPlacementSave();
  }

  @override
  void onWindowRestore() {
    if (_isClosing) {
      return;
    }
    _runBackground(_synchronizeRestoredState());
  }

  @override
  void onWindowUnmaximize() {
    if (_isClosing) {
      return;
    }
    _isMaximized.value = false;
    _scheduleNormalPlacementSave();
  }

  @override
  void dispose() {
    _placementSaveDebounce?.cancel();
    windowManager.removeListener(this);
    _isMaximized.dispose();
  }

  void _scheduleNormalPlacementSave() {
    if (_isClosing) {
      return;
    }
    _placementSaveDebounce?.cancel();
    _placementSaveDebounce = Timer(
      const Duration(milliseconds: 250),
      () => _runBackground(_captureNormalPlacement()),
    );
  }

  void _beginClosing() {
    _isClosing = true;
    _placementSaveDebounce?.cancel();
  }

  void _runBackground(Future<void> operation) {
    unawaited(_ignoreFailure(operation));
  }

  Future<void> _ignoreFailure(Future<void> operation) async {
    try {
      await operation;
    } on Object {
      // UI preferences are best-effort and must not terminate the window.
    }
  }

  Future<void> _captureNormalPlacement() async {
    if (await windowManager.isMaximized() ||
        await windowManager.isMinimized()) {
      return;
    }
    final position = await windowManager.getPosition();
    final size = await windowManager.getSize();
    final placement = AmeWindowPlacement(
      left: position.dx,
      top: position.dy,
      width: size.width,
      height: size.height,
      isMaximized: false,
    );
    _normalPlacement = placement;
    await _preferenceStore.saveWindowPlacement(placement);
  }

  Future<void> _saveKnownPlacement({required bool isMaximized}) async {
    final placement = _normalPlacement;
    if (placement == null) {
      return;
    }
    await _preferenceStore.saveWindowPlacement(
      placement.copyWith(isMaximized: isMaximized),
    );
  }

  Future<void> _synchronizeRestoredState() async {
    final isMaximized = await windowManager.isMaximized();
    _isMaximized.value = isMaximized;
    if (isMaximized) {
      await _saveKnownPlacement(isMaximized: true);
      return;
    }
    _scheduleNormalPlacementSave();
  }
}

Future<WindowManagerActions> initializeAmeWindow(
  AmeWindowPreferenceStore preferenceStore,
) async {
  await windowManager.ensureInitialized();
  AmeWindowPlacement? savedPlacement;
  try {
    savedPlacement = await preferenceStore.loadWindowPlacement();
  } on Object {
    savedPlacement = null;
  }
  final visibleBounds = await _loadVisibleScreenBounds();
  final restoredPlacement = normalizeAmeWindowPlacement(
    savedPlacement,
    visibleScreenBounds: visibleBounds,
    minimumSize: _minimumWindowSize,
  );
  final options = WindowOptions(
    size: restoredPlacement == null
        ? null
        : Size(restoredPlacement.width, restoredPlacement.height),
    minimumSize: _minimumWindowSize,
    backgroundColor: Colors.transparent,
    title: "Cedarflake Ame",
    titleBarStyle: TitleBarStyle.hidden,
    windowButtonVisibility: false,
  );
  AmeWindowPlacement? initialNormalPlacement;
  await windowManager.waitUntilReadyToShow(options, () async {
    if (restoredPlacement != null) {
      await windowManager.setPosition(
        Offset(restoredPlacement.left, restoredPlacement.top),
      );
    }
    final position = await windowManager.getPosition();
    final size = await windowManager.getSize();
    initialNormalPlacement = AmeWindowPlacement(
      left: position.dx,
      top: position.dy,
      width: size.width,
      height: size.height,
      isMaximized: false,
    );
    if (savedPlacement?.isMaximized ?? false) {
      await windowManager.maximize();
    }
    await windowManager.show();
    await windowManager.focus();
  });
  final actions = WindowManagerActions(
    preferenceStore,
    initialNormalPlacement: initialNormalPlacement,
  );
  await actions.initialize();
  return actions;
}

Future<List<Rect>> _loadVisibleScreenBounds() async {
  try {
    final primary = await screenRetriever.getPrimaryDisplay();
    final displays = await screenRetriever.getAllDisplays();
    final orderedDisplays = [
      primary,
      ...displays.where((display) => display.id != primary.id),
    ];
    return orderedDisplays
        .map((display) {
          final position = display.visiblePosition ?? Offset.zero;
          final size = display.visibleSize ?? display.size;
          return position & size;
        })
        .toList(growable: false);
  } on Object {
    return const [];
  }
}
