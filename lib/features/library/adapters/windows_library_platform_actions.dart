import "dart:io";

import "package:flutter/foundation.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../application/library_platform_actions.dart";

class WindowsLibraryPlatformActions implements LibraryPlatformActions {
  const WindowsLibraryPlatformActions()
    : _launchExplorer = _launchDetachedExplorer;

  @visibleForTesting
  WindowsLibraryPlatformActions.withLauncher(this._launchExplorer);

  final Future<void> Function(List<String> arguments) _launchExplorer;

  @override
  Future<void> copyText(String value) {
    return Clipboard.setData(ClipboardData(text: value));
  }

  @override
  Future<void> revealDirectory(String path) async {
    await _revealPath(path);
  }

  @override
  Future<void> revealLibraryFolder(String rootPath, String relativePath) async {
    final separator = Platform.pathSeparator;
    final root = rootPath.replaceAll(RegExp(r"[\\/]+$"), "");
    final relative = relativePath
        .split(RegExp(r"[\\/]"))
        .where((component) => component.isNotEmpty)
        .join(separator);
    await _revealPath("$root$separator$relative");
  }

  @override
  Future<void> revealFile(String path) async {
    await _revealPath(path);
  }

  Future<void> _revealPath(String path) {
    final shellPath = _explorerCompatiblePath(path);
    return _launchExplorer(["/select,$shellPath"]);
  }
}

String _explorerCompatiblePath(String path) {
  if (path.startsWith(r"\\?\UNC\")) {
    return r"\\" + path.substring(8);
  }
  if (path.startsWith(r"\\?\")) {
    return path.substring(4);
  }
  return path;
}

Future<void> _launchDetachedExplorer(List<String> arguments) async {
  if (!Platform.isWindows) {
    throw UnsupportedError("File Explorer actions require Windows");
  }
  await Process.start(
    "explorer.exe",
    arguments,
    mode: ProcessStartMode.detached,
  );
}

final libraryPlatformActionsProvider = Provider<LibraryPlatformActions>((ref) {
  return const WindowsLibraryPlatformActions();
});
