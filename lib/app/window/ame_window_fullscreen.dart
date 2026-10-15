import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "ame_window_actions.dart";

abstract interface class AmeFullscreenHost {
  AmeWindowFullscreen get fullscreen;
}

final ameWindowFullscreenProvider = Provider<AmeWindowFullscreen?>((ref) {
  final actions = ref.watch(ameWindowActionsProvider);
  return actions is AmeFullscreenHost
      ? (actions as AmeFullscreenHost).fullscreen
      : null;
});

/// Serializes native transitions and protects normal placement during their events.
class AmeWindowFullscreen extends ChangeNotifier {
  AmeWindowFullscreen({
    required this._setFullscreen,
    required this._readFullscreen,
    required this._capturePlacement,
  });

  final Future<void> Function(bool) _setFullscreen;
  final Future<bool> Function() _readFullscreen;
  final Future<void> Function() _capturePlacement;
  bool _isFullscreen = false;
  bool _isDisposed = false;
  bool _isApplying = false;
  Future<void>? _operation;
  int _generation = 0;

  bool get isFullscreen => _isFullscreen;
  bool get isChanging => _operation != null;
  bool get protectsPlacement => _isFullscreen || _isApplying;

  Future<void> toggle() => setEnabled(!isFullscreen);

  Future<void> setEnabled(bool enabled) async {
    while (_operation != null) {
      try {
        await _operation;
      } on Object {
        // A later exit request must still be able to restore the native window.
      }
    }
    if (_isDisposed || _isFullscreen == enabled) return;
    final operation = _change(enabled);
    _operation = operation;
    notifyListeners();
    try {
      await operation;
    } finally {
      if (identical(_operation, operation)) _operation = null;
      if (!_isDisposed) notifyListeners();
    }
  }

  Future<void> _change(bool enabled) async {
    if (enabled) await _capturePlacement();
    if (_isDisposed) return;
    _generation++;
    _isApplying = true;
    try {
      await _setFullscreen(enabled);
      final actual = await _readFullscreen();
      if (!_isDisposed) _isFullscreen = actual;
    } finally {
      _isApplying = false;
    }
  }

  void synchronize(bool enabled) {
    if (_isDisposed) return;
    _generation++;
    _isFullscreen = enabled;
    notifyListeners();
  }

  /// An ordinary bounds read may finish after entry; its old authority then expires.
  int get placementGeneration => _generation;

  bool canSavePlacement(int generation) =>
      !_isDisposed && !protectsPlacement && generation == _generation;

  @override
  void dispose() {
    _isDisposed = true;
    super.dispose();
  }
}
