import "dart:async";
import "dart:io";

import "package:flutter/services.dart";

import "../../domain/diagnostic_report.dart";

class PlatformDiagnosticEnvironment {
  const PlatformDiagnosticEnvironment();

  static const _channel = MethodChannel("cedarflake_ame/diagnostics");

  Future<
    ({
      List<DiagnosticFact> facts,
      List<DiagnosticGraphicsAdapter> graphicsAdapters,
    })
  >
  collect() async {
    final hardware = await _readHardware();
    final architecture =
        RegExp(
          r"(?:windows|linux|macos)_(x64|arm64|ia32|arm)",
        ).firstMatch(Platform.version)?.group(1) ??
        "unknown";
    final operatingSystemVersion = Platform.operatingSystemVersion
        .replaceAll(RegExp(r"[^A-Za-z0-9_. :+()\-]"), " ")
        .trim();
    final scale = ServicesBinding
        .instance
        .platformDispatcher
        .implicitView
        ?.devicePixelRatio;
    final memory = hardware["physicalMemoryBytes"];
    final adapters = <DiagnosticGraphicsAdapter>[];
    final rawAdapters = hardware["graphicsAdapters"];
    if (rawAdapters is List<Object?>) {
      for (final adapter in rawAdapters.take(8)) {
        if (adapter is! Map<Object?, Object?>) continue;
        final version = adapter["driverVersion"];
        adapters.add(
          DiagnosticGraphicsAdapter(
            model: _hardwareModel(adapter["model"]),
            driverVersion:
                version is String && RegExp(r"^[0-9.]{1,80}$").hasMatch(version)
                ? version
                : "unavailable",
          ),
        );
      }
    }
    final facts = [
      DiagnosticFact(DiagnosticField.operatingSystem, Platform.operatingSystem),
      DiagnosticFact(
        DiagnosticField.operatingSystemVersion,
        operatingSystemVersion.isEmpty
            ? "unknown"
            : operatingSystemVersion.substring(
                0,
                operatingSystemVersion.length.clamp(0, 160),
              ),
      ),
      DiagnosticFact(DiagnosticField.architecture, architecture),
      DiagnosticFact(
        DiagnosticField.dartVersion,
        Platform.version.split(" ").first,
      ),
      DiagnosticFact(
        DiagnosticField.flutterVersion,
        FlutterVersion.version ?? "unavailable",
      ),
      DiagnosticFact(
        DiagnosticField.flutterEngineRevision,
        FlutterVersion.engineRevision ?? "unavailable",
      ),
      DiagnosticFact(
        DiagnosticField.cpuModel,
        _hardwareModel(hardware["cpuModel"]),
      ),
      DiagnosticFact(
        DiagnosticField.physicalMemoryBytes,
        memory is int && memory > 0 ? memory.toString() : "unavailable",
      ),
      DiagnosticFact(
        DiagnosticField.displayScalePercent,
        scale != null && scale.isFinite && scale > 0
            ? (scale * 100).toStringAsFixed(2)
            : "unavailable",
      ),
      DiagnosticFact(
        DiagnosticField.graphicsAdapterState,
        switch (hardware["graphicsAdapterState"]) {
          "available" => "available",
          "partial" => "partial",
          "truncated" => "truncated",
          _ => "unavailable",
        },
      ),
    ];
    return (facts: facts, graphicsAdapters: adapters);
  }

  Future<Map<Object?, Object?>> _readHardware() async {
    try {
      return await _channel
              .invokeMapMethod<Object?, Object?>("getEnvironment")
              .timeout(const Duration(seconds: 2)) ??
          const {};
    } on MissingPluginException {
      return const {};
    } on PlatformException {
      return const {};
    } on TimeoutException {
      return const {};
    }
  }
}

String _hardwareModel(Object? value) {
  if (value is! String) return "unavailable";
  final model = value.trim();
  return isDiagnosticHardwareModel(model) ? model : "unavailable";
}
