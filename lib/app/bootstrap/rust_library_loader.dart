import "dart:io";

import "package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart";

import "../../src/rust/frb_generated.dart";

const rustLibraryFileName = "rust_lib_cedarflake_ame.dll";

String packagedRustLibraryPath(String executablePath) {
  final executableDirectory = File(executablePath).parent.path;
  return "$executableDirectory${Platform.pathSeparator}$rustLibraryFileName";
}

Future<void> initializeRustLibrary() {
  if (!Platform.isWindows) {
    return RustLib.init();
  }

  final libraryPath = packagedRustLibraryPath(Platform.resolvedExecutable);
  return RustLib.init(
    externalLibrary: ExternalLibrary.open(
      libraryPath,
      debugInfo: "Rust library next to the Cedarflake Ame executable",
    ),
  );
}
