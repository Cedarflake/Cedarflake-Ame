import "dart:io";

import "../../domain/diagnostic_report.dart";

class PlatformDiagnosticEnvironment {
  const PlatformDiagnosticEnvironment();

  List<DiagnosticFact> facts() {
    final architecture =
        RegExp(
          r"(?:windows|linux|macos)_(x64|arm64|ia32|arm)",
        ).firstMatch(Platform.version)?.group(1) ??
        "unknown";
    final operatingSystemVersion = Platform.operatingSystemVersion
        .replaceAll(RegExp(r"[^A-Za-z0-9_. :+()\-]"), " ")
        .trim();
    return [
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
    ];
  }
}
