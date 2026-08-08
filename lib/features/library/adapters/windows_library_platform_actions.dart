import "dart:io";

import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../application/library_platform_actions.dart";

class WindowsLibraryPlatformActions implements LibraryPlatformActions {
  const WindowsLibraryPlatformActions();

  @override
  Future<void> copyText(String value) {
    return Clipboard.setData(ClipboardData(text: value));
  }

  @override
  Future<void> openDirectory(String path) async {
    await _startExplorer([path]);
  }

  @override
  Future<void> openLibraryFolder(String rootPath, String relativePath) async {
    final separator = Platform.pathSeparator;
    final root = rootPath.replaceAll(RegExp(r"[\\/]+$"), "");
    final relative = relativePath
        .split(RegExp(r"[\\/]"))
        .where((component) => component.isNotEmpty)
        .join(separator);
    await openDirectory("$root$separator$relative");
  }

  @override
  Future<void> revealFile(String path) async {
    await _startExplorer(["/select,", path]);
  }

  Future<void> _startExplorer(List<String> arguments) async {
    if (!Platform.isWindows) {
      throw UnsupportedError("File Explorer actions require Windows");
    }
    await Process.start(
      "explorer.exe",
      arguments,
      mode: ProcessStartMode.detached,
    );
  }
}

final libraryPlatformActionsProvider = Provider<LibraryPlatformActions>((ref) {
  return const WindowsLibraryPlatformActions();
});
