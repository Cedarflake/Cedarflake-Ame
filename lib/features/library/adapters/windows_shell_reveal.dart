import "dart:ffi";
import "dart:io";
import "dart:isolate";

import "package:flutter/services.dart";

Future<void> revealWindowsShellItem(String path) async {
  if (!Platform.isWindows) {
    throw UnsupportedError("File Explorer actions require Windows");
  }
  if (path.isEmpty || path.contains("\u0000")) {
    throw ArgumentError.value(path, "path", "Expected a nonempty shell path");
  }
  final result = await Isolate.run(() => _revealItem(path));
  if (result < 0) {
    throw PlatformException(
      code: "windows_shell_reveal_failed",
      message: "Windows could not select the requested item",
      details: {"hresult": result.toUnsigned(32).toRadixString(16)},
    );
  }
}

int _revealItem(String path) {
  const apartmentThreaded = 2;
  const outOfMemory = -2147024882;
  const invalidShellItem = -2147467259;
  final ole32 = DynamicLibrary.open("ole32.dll");
  final shell32 = DynamicLibrary.open("shell32.dll");
  final initialize = ole32
      .lookupFunction<
        Int32 Function(Pointer<Void>, Uint32),
        int Function(Pointer<Void>, int)
      >("CoInitializeEx");
  final uninitialize = ole32.lookupFunction<Void Function(), void Function()>(
    "CoUninitialize",
  );
  final allocate = ole32
      .lookupFunction<
        Pointer<Void> Function(UintPtr),
        Pointer<Void> Function(int)
      >("CoTaskMemAlloc");
  final free = ole32
      .lookupFunction<
        Void Function(Pointer<Void>),
        void Function(Pointer<Void>)
      >("CoTaskMemFree");
  final parse = shell32
      .lookupFunction<
        Int32 Function(
          Pointer<Uint16>,
          Pointer<Void>,
          Pointer<Pointer<Void>>,
          Uint32,
          Pointer<Uint32>,
        ),
        int Function(
          Pointer<Uint16>,
          Pointer<Void>,
          Pointer<Pointer<Void>>,
          int,
          Pointer<Uint32>,
        )
      >("SHParseDisplayName");
  final select = shell32
      .lookupFunction<
        Int32 Function(Pointer<Void>, Uint32, Pointer<Pointer<Void>>, Uint32),
        int Function(Pointer<Void>, int, Pointer<Pointer<Void>>, int)
      >("SHOpenFolderAndSelectItems");

  // All COM calls remain synchronous on this worker isolate's thread.
  final initialized = initialize(nullptr, apartmentThreaded);
  if (initialized < 0) return initialized;
  try {
    final units = path.codeUnits;
    final name = allocate((units.length + 1) * sizeOf<Uint16>()).cast<Uint16>();
    if (name == nullptr) return outOfMemory;
    try {
      final buffer = name.asTypedList(units.length + 1);
      buffer.setAll(0, units);
      buffer[units.length] = 0;
      final item = allocate(sizeOf<Pointer<Void>>()).cast<Pointer<Void>>();
      if (item == nullptr) return outOfMemory;
      item.value = nullptr;
      try {
        final parsed = parse(name, nullptr, item, 0, nullptr);
        if (parsed < 0) return parsed;
        if (item.value == nullptr) return invalidShellItem;
        // A zero item count selects this absolute item in its parent folder.
        return select(item.value, 0, nullptr, 0);
      } finally {
        free(item.value);
        free(item.cast());
      }
    } finally {
      free(name.cast());
    }
  } finally {
    uninitialize();
  }
}
