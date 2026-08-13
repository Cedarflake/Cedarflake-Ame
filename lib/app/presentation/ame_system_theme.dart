import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";

const ameFallbackSeedColor = Color(0xFF0B57D0);

const _systemThemeChannel = MethodChannel("cedarflake_ame/system_theme");
const _getAccentColorMethod = "getAccentColor";
const _accentColorChangedMethod = "accentColorChanged";

typedef AmeSystemThemeWidgetBuilder =
    Widget Function(BuildContext context, Color seedColor);

Future<Color> loadAmeSystemSeedColor() async {
  try {
    final packedColor = await _systemThemeChannel.invokeMethod<Object?>(
      _getAccentColorMethod,
    );
    return _readAccentColor(packedColor) ?? ameFallbackSeedColor;
  } on Object {
    return ameFallbackSeedColor;
  }
}

class AmeSystemThemeBuilder extends StatefulWidget {
  const AmeSystemThemeBuilder({required this.builder, super.key});

  final AmeSystemThemeWidgetBuilder builder;

  @override
  State<AmeSystemThemeBuilder> createState() => _AmeSystemThemeBuilderState();
}

class _AmeSystemThemeBuilderState extends State<AmeSystemThemeBuilder> {
  Color _seedColor = ameFallbackSeedColor;
  int _accentRevision = 0;

  @override
  void initState() {
    super.initState();
    _systemThemeChannel.setMethodCallHandler(_handleMethodCall);
    unawaited(_loadSeedColor());
  }

  @override
  void dispose() {
    _systemThemeChannel.setMethodCallHandler(null);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.builder(context, _seedColor);

  Future<void> _loadSeedColor() async {
    final requestedRevision = _accentRevision;
    final seedColor = await loadAmeSystemSeedColor();
    if (!mounted ||
        requestedRevision != _accentRevision ||
        seedColor == _seedColor) {
      return;
    }
    setState(() => _seedColor = seedColor);
  }

  Future<Object?> _handleMethodCall(MethodCall call) async {
    if (call.method != _accentColorChangedMethod) {
      return null;
    }
    _accentRevision += 1;
    final seedColor = _readAccentColor(call.arguments);
    if (!mounted || seedColor == null || seedColor == _seedColor) {
      return null;
    }
    setState(() => _seedColor = seedColor);
    return null;
  }
}

Color? _readAccentColor(Object? packedColor) {
  if (packedColor is! num || !packedColor.isFinite) {
    return null;
  }
  final value = packedColor.toInt();
  if (value < 0 || value > 0xFFFFFFFF) {
    return null;
  }
  return Color.fromARGB(
    255,
    (value >> 16) & 0xFF,
    (value >> 8) & 0xFF,
    value & 0xFF,
  );
}
