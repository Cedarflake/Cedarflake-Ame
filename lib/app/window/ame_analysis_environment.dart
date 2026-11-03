import "package:flutter/services.dart";
import "package:flutter/widgets.dart";

class AmeAnalysisEnvironment with WidgetsBindingObserver {
  AmeAnalysisEnvironment({
    required this._onSuspendedChanged,
    required this._onMemoryPressure,
    required this._onError,
    this._powerChannel = const MethodChannel("cedarflake_ame/system_power"),
    WidgetsBinding? binding,
  }) : _binding = binding ?? WidgetsBinding.instance;

  final void Function(bool suspended) _onSuspendedChanged;
  final VoidCallback _onMemoryPressure;
  final void Function(Object error, StackTrace stackTrace) _onError;
  final MethodChannel _powerChannel;
  final WidgetsBinding _binding;
  _PowerSnapshot? _power;
  bool _applicationSuspended = false;
  bool? _reportedSuspended;
  bool _started = false;
  bool _disposed = false;
  bool _failed = false;

  Future<void> start() async {
    if (_started || _disposed) {
      return;
    }
    _started = true;
    _binding.addObserver(this);
    _applicationSuspended = _isSuspended(_binding.lifecycleState);
    _publish();
    _powerChannel.setMethodCallHandler(_handlePowerEvent);
    try {
      _accept(await _powerChannel.invokeMethod<Object?>("getState"));
    } on Object catch (error, stackTrace) {
      _fail(error, stackTrace);
    }
  }

  Future<Object?> _handlePowerEvent(MethodCall call) async {
    if (call.method != "stateChanged") {
      throw MissingPluginException("Unknown system power event");
    }
    try {
      _accept(call.arguments);
    } on Object catch (error, stackTrace) {
      _fail(error, stackTrace);
    }
    return null;
  }

  void _accept(Object? value) {
    if (_disposed || _failed) {
      return;
    }
    final next = _PowerSnapshot.parse(value);
    final previous = _power;
    if (previous != null && next.revision <= previous.revision) {
      if (next.revision == previous.revision &&
          next.suspended != previous.suspended) {
        throw const FormatException("Conflicting system power snapshot");
      }
      return;
    }
    if (!next.suspended && next.revision > 0) {
      // Fence an old turn even when Windows could not deliver its suspend event.
      _report(true);
    }
    _power = next;
    _publish();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _applicationSuspended = _isSuspended(state);
    _publish();
  }

  @override
  void didHaveMemoryPressure() {
    if (!_disposed) {
      _onMemoryPressure();
    }
  }

  void _publish() => _report(
    _failed || _power == null || _power!.suspended || _applicationSuspended,
  );

  void _report(bool suspended) {
    if (_disposed || _reportedSuspended == suspended) {
      return;
    }
    _reportedSuspended = suspended;
    _onSuspendedChanged(suspended);
  }

  void _fail(Object error, StackTrace stackTrace) {
    if (_disposed || _failed) {
      return;
    }
    _failed = true;
    _publish();
    _onError(error, stackTrace);
  }

  void dispose() {
    if (_disposed) {
      return;
    }
    _disposed = true;
    if (_started) {
      _binding.removeObserver(this);
      _powerChannel.setMethodCallHandler(null);
    }
  }

  static bool _isSuspended(AppLifecycleState? state) =>
      state == AppLifecycleState.paused || state == AppLifecycleState.detached;
}

class _PowerSnapshot {
  const _PowerSnapshot(this.revision, this.suspended);

  factory _PowerSnapshot.parse(Object? value) {
    if (value case {
      "revision": int revision,
      "suspended": bool suspended,
    } when revision >= 0) {
      return _PowerSnapshot(revision, suspended);
    }
    throw const FormatException("Invalid system power snapshot");
  }

  final int revision;
  final bool suspended;
}
