import "dart:io";

import "package:window_manager/window_manager.dart";

import "../../application/ports/library_file_operations.dart";

class WindowsFileOperationPlatform implements LibraryFileOperationPlatform {
  const WindowsFileOperationPlatform();

  @override
  Future<BigInt> ownerWindow() async =>
      BigInt.from(await windowManager.getId());

  @override
  Future<void> openRecycleBin() async {
    await Process.start("explorer.exe", [
      "shell:RecycleBinFolder",
    ], mode: ProcessStartMode.detached);
  }
}
