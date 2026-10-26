import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter/material.dart";
import "package:screen_retriever/screen_retriever.dart";
import "package:window_manager/window_manager.dart";

import "ame_window_actions.dart";
import "ame_window_placement.dart";
import "ame_window_fullscreen.dart";
import "ame_shutdown_coordinator.dart";

class WindowManagerActions
    with WindowListener
    implements AmeWindowActions, AmeFullscreenHost {
  WindowManagerActions(
    this._preferenceStore, {
    required this._shutdownCoordinator,
    Duration maximumShutdownDuration = const Duration(seconds: 6),
    Future<void> Function()? hideWindow,
    Future<void> Function()? destroyWindow,
    AmeWindowPlacement? initialNormalPlacement,
  }) : _shutdownTimeout = maximumShutdownDuration,
       _hideWindow = hideWindow ?? windowManager.hide,
       _destroyWindow = destroyWindow ?? windowManager.destroy,
       _normalPlacement = initialNormalPlacement?.copyWith(isMaximized: false);

  final AmeWindowPreferenceStore _preferenceStore;
  final AmeShutdownCoordinator _shutdownCoordinator;
  final Duration _shutdownTimeout;
  final Future<void> Function() _hideWindow;
  final Future<void> Function() _destroyWindow;
  final ValueNotifier<bool> _isMaximized = ValueNotifier(false);
  AmeWindowPlacement? _normalPlacement;
  Timer? _placementSaveDebounce;
  bool _isClosing = false;
  Future<void>? _closeOperation;

  @override
  late final fullscreen = AmeWindowFullscreen(
    setFullscreen: windowManager.setFullScreen,
    readFullscreen: windowManager.isFullScreen,
    capturePlacement: _captureNormalPlacement,
  );

  @override
  ValueListenable<bool> get isMaximized => _isMaximized;

  Future<void> initialize() async {
    windowManager.addListener(this);
    await windowManager.setPreventClose(true);
    _isMaximized.value = await windowManager.isMaximized();
    if (!_isMaximized.value && !await windowManager.isMinimized()) {
      await _captureNormalPlacement();
    }
  }

  @override
  Future<void> minimize() => windowManager.minimize();

  @override
  Future<void> toggleMaximize() async {
    if (fullscreen.protectsPlacement) return;
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
    return _requestClose();
  }

  @override
  void onWindowClose() {
    unawaited(_requestClose());
  }

  @override
  void onWindowMaximize() {
    if (_isClosing || fullscreen.protectsPlacement) {
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
    if (_isClosing || fullscreen.protectsPlacement) {
      return;
    }
    _runBackground(_synchronizeRestoredState());
  }

  @override
  void onWindowUnmaximize() {
    if (_isClosing || fullscreen.protectsPlacement) {
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
    fullscreen.dispose();
  }

  @override
  void onWindowEnterFullScreen() {
    _placementSaveDebounce?.cancel();
    fullscreen.synchronize(true);
  }

  @override
  void onWindowLeaveFullScreen() => fullscreen.synchronize(false);

  void _scheduleNormalPlacementSave() {
    if (_isClosing || fullscreen.protectsPlacement) {
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

  Future<void> _requestClose() {
    return _closeOperation ??= _closeAfterShutdown();
  }

  Future<void> _closeAfterShutdown() async {
    _beginClosing();
    try {
      await _hideWindow();
    } on Object {
      // Shutdown must continue even if the platform cannot hide the window.
    }
    try {
      await _shutdownCoordinator.shutdown().timeout(_shutdownTimeout);
    } on Object {
      // A bounded shutdown must not leave the desktop window trapped open.
    }
    await _destroyWindow();
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
    final generation = fullscreen.placementGeneration;
    if (await windowManager.isMaximized() ||
        await windowManager.isMinimized()) {
      return;
    }
    final position = await windowManager.getPosition();
    final size = await windowManager.getSize();
    if (_isClosing || !fullscreen.canSavePlacement(generation)) return;
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
    if (_isClosing || fullscreen.protectsPlacement) return;
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
  AmeShutdownCoordinator shutdownCoordinator,
) async {
  await windowManager.ensureInitialized();
  AmeWindowPlacement? savedPlacement;
  try {
    savedPlacement = await preferenceStore.loadWindowPlacement();
  } on Object {
    savedPlacement = null;
  }
  final visibleBounds = await _loadVisibleScreenBounds();
  final minimumSize = resolveAmeMinimumWindowSize(
    savedPlacement,
    visibleScreenBounds: visibleBounds,
  );
  final restoredPlacement = normalizeAmeWindowPlacement(
    savedPlacement,
    visibleScreenBounds: visibleBounds,
    minimumSize: minimumSize,
  );
  final initialNormalPlacement = await prepareAmeWindowBeforeShow(
    window: const _WindowManagerBootstrapActions(),
    restoredPlacement: restoredPlacement,
    shouldMaximize: savedPlacement?.isMaximized ?? false,
    minimumSize: minimumSize,
  );
  final actions = WindowManagerActions(
    preferenceStore,
    shutdownCoordinator: shutdownCoordinator,
    initialNormalPlacement: initialNormalPlacement,
  );
  await actions.initialize();
  return actions;
}

@visibleForTesting
abstract interface class AmeWindowBootstrapActions {
  Future<void> waitUntilReadyToShow(WindowOptions options);

  Future<void> setPosition(Offset position);

  Future<Offset> getPosition();

  Future<Size> getSize();

  Future<void> maximize();

  Future<void> show();

  Future<void> focus();
}

class _WindowManagerBootstrapActions implements AmeWindowBootstrapActions {
  const _WindowManagerBootstrapActions();

  @override
  Future<void> waitUntilReadyToShow(WindowOptions options) {
    return windowManager.waitUntilReadyToShow(options);
  }

  @override
  Future<void> setPosition(Offset position) {
    return windowManager.setPosition(position);
  }

  @override
  Future<Offset> getPosition() => windowManager.getPosition();

  @override
  Future<Size> getSize() => windowManager.getSize();

  @override
  Future<void> maximize() => windowManager.maximize();

  @override
  Future<void> show() => windowManager.show();

  @override
  Future<void> focus() => windowManager.focus();
}

@visibleForTesting
Future<AmeWindowPlacement> prepareAmeWindowBeforeShow({
  required AmeWindowBootstrapActions window,
  required AmeWindowPlacement? restoredPlacement,
  required bool shouldMaximize,
  Size minimumSize = ameMinimumWindowSize,
}) async {
  final options = WindowOptions(
    size: restoredPlacement == null
        ? null
        : Size(restoredPlacement.width, restoredPlacement.height),
    minimumSize: minimumSize,
    backgroundColor: Colors.transparent,
    title: "Cedarflake Ame",
    titleBarStyle: TitleBarStyle.hidden,
    windowButtonVisibility: false,
  );
  await window.waitUntilReadyToShow(options);
  if (restoredPlacement != null) {
    await window.setPosition(
      Offset(restoredPlacement.left, restoredPlacement.top),
    );
  }
  final position = await window.getPosition();
  final size = await window.getSize();
  final initialNormalPlacement = AmeWindowPlacement(
    left: position.dx,
    top: position.dy,
    width: size.width,
    height: size.height,
    isMaximized: false,
  );
  if (shouldMaximize) {
    await window.maximize();
  }
  return initialNormalPlacement;
}

Future<void> showAmeWindowAfterFirstFrame({
  AmeWindowBootstrapActions window = const _WindowManagerBootstrapActions(),
  Future<void>? firstFrameRasterized,
  Duration firstFrameTimeout = const Duration(seconds: 5),
}) async {
  try {
    await (firstFrameRasterized ??
            WidgetsBinding.instance.waitUntilFirstFrameRasterized)
        .timeout(firstFrameTimeout);
  } on Object {
    // Showing the mounted shell is safer than leaving a failed renderer invisible.
  }
  await window.show();
  await window.focus();
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
