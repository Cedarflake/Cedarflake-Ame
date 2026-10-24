import "dart:ffi";
import "dart:io";

class SourceReadBlock {
  SourceReadBlock._(this._handle, this._closeHandle, this._lastError);

  int? _handle;
  final int Function(int) _closeHandle;
  final int Function() _lastError;

  static Future<SourceReadBlock> acquire(String fixturePath) async {
    if (!Platform.isWindows || fixturePath.contains("\u0000")) {
      throw ArgumentError("Expected a Windows native fixture path");
    }
    final source = File(fixturePath).absolute;
    final canonical = await source.resolveSymbolicLinks();
    final build = await Directory(
      "${Directory.current.path}/build",
    ).resolveSymbolicLinks();
    final prefix = "${_normalized(build)}\\";
    final path = _normalized(canonical);
    if (path != _normalized(source.path) ||
        !path.startsWith(prefix) ||
        !RegExp(
          r"^integration-fixture-(?:second-)?[0-9]+\\",
        ).hasMatch(path.substring(prefix.length))) {
      throw ArgumentError("The source must be an owned native fixture");
    }

    final kernel = DynamicLibrary.open("kernel32.dll");
    final ole = DynamicLibrary.open("ole32.dll");
    final allocate = ole
        .lookupFunction<
          Pointer<Void> Function(UintPtr),
          Pointer<Void> Function(int)
        >("CoTaskMemAlloc");
    final free = ole
        .lookupFunction<
          Void Function(Pointer<Void>),
          void Function(Pointer<Void>)
        >("CoTaskMemFree");
    final open = kernel
        .lookupFunction<
          IntPtr Function(
            Pointer<Uint16>,
            Uint32,
            Uint32,
            Pointer<Void>,
            Uint32,
            Uint32,
            IntPtr,
          ),
          int Function(Pointer<Uint16>, int, int, Pointer<Void>, int, int, int)
        >("CreateFileW");
    final close = kernel
        .lookupFunction<Int32 Function(IntPtr), int Function(int)>(
          "CloseHandle",
        );
    final lastError = kernel.lookupFunction<Uint32 Function(), int Function()>(
      "GetLastError",
    );
    final units = canonical.codeUnits;
    final name = allocate((units.length + 1) * sizeOf<Uint16>()).cast<Uint16>();
    if (name == nullptr) {
      throw StateError("Could not allocate the fixture path");
    }
    try {
      final buffer = name.asTypedList(units.length + 1);
      buffer.setAll(0, units);
      buffer[units.length] = 0;
      const genericRead = 0x80000000;
      const noSharing = 0;
      const openExisting = 3;
      const normalAttributes = 0x80;
      final handle = open(
        name,
        genericRead,
        noSharing,
        nullptr,
        openExisting,
        normalAttributes,
        0,
      );
      if (handle == -1) {
        throw StateError("Could not hold the fixture source: ${lastError()}");
      }
      return SourceReadBlock._(handle, close, lastError);
    } finally {
      free(name.cast());
    }
  }

  Future<void> release() async {
    final handle = _handle;
    if (handle == null) return;
    if (_closeHandle(handle) == 0) {
      throw StateError("Could not release the fixture source: ${_lastError()}");
    }
    _handle = null;
  }

  static String _normalized(String path) =>
      (path.startsWith(r"\\?\") ? path.substring(4) : path)
          .replaceAll("/", "\\")
          .toLowerCase();
}
