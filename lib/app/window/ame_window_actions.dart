import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

abstract interface class AmeWindowActions {
  ValueListenable<bool> get isMaximized;

  Future<void> minimize();

  Future<void> toggleMaximize();

  Future<void> close();

  void dispose();
}

final ameWindowActionsProvider = Provider<AmeWindowActions>((ref) {
  final actions = _UnavailableWindowActions();
  ref.onDispose(actions.dispose);
  return actions;
});

class _UnavailableWindowActions implements AmeWindowActions {
  final ValueNotifier<bool> _isMaximized = ValueNotifier(false);

  @override
  ValueListenable<bool> get isMaximized => _isMaximized;

  @override
  Future<void> close() async {}

  @override
  void dispose() {
    _isMaximized.dispose();
  }

  @override
  Future<void> minimize() async {}

  @override
  Future<void> toggleMaximize() async {}
}
