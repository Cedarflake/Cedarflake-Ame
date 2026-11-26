import "dart:async";
import "dart:convert";
import "dart:io";

import "package:path_provider/path_provider.dart";

import "../../domain/diagnostic_report.dart";
import "diagnostic_encoding.dart";

class PlatformDiagnosticJournal {
  PlatformDiagnosticJournal({Future<Directory> Function()? directory})
    : _directory = directory ?? _defaultDirectory;

  final Future<Directory> Function() _directory;
  final List<DiagnosticEvent> _events = [];
  Directory? _root;
  Timer? _flushTimer;
  Future<void> _writing = Future.value();
  Future<void>? _initializing;
  Future<void>? _closing;
  bool _closed = false;
  bool _available = true;
  bool _dirty = false;
  bool _flushing = false;
  bool _initialized = false;
  bool get available => _initialized && _available;

  static Future<Directory> _defaultDirectory() async {
    final support = await getApplicationSupportDirectory();
    return Directory("${support.path}${Platform.pathSeparator}diagnostics");
  }

  Future<void> initialize() => _initializing ??= _initialize();

  Future<void> _initialize() async {
    Directory? storage;
    try {
      final root = await _directory();
      if (_closed) return;
      if (await FileSystemEntity.type(root.path, followLinks: false) ==
          FileSystemEntityType.link) {
        throw const FileSystemException("Linked diagnostic directory");
      }
      await root.create(recursive: true);
      if (_closed) return;
      storage = root;
      final file = File("${root.path}${Platform.pathSeparator}events.json");
      final type = await FileSystemEntity.type(file.path, followLinks: false);
      if (type == FileSystemEntityType.notFound) return;
      if (type != FileSystemEntityType.file) {
        throw const FormatException("Invalid diagnostic journal");
      }
      final handle = await file.open();
      late final List<int> bytes;
      try {
        bytes = await handle.read(diagnosticByteLimit + 1);
      } finally {
        await handle.close();
      }
      if (bytes.length > diagnosticByteLimit) {
        throw const FormatException("Oversized diagnostic journal");
      }
      final Object? decoded = jsonDecode(utf8.decode(bytes));
      if (decoded is! List<Object?> || decoded.length > 256) {
        throw const FormatException("Invalid diagnostic journal");
      }
      final retained = decoded.map(decodeDiagnosticEvent).toList();
      if (!_closed) _events.insertAll(0, retained);
      _trim();
    } on Object {
      _available = false;
    } finally {
      _initialized = true;
      if (!_closed && storage != null) {
        _root = storage;
        if (_events.isNotEmpty) _flush();
      }
    }
  }

  void record(DiagnosticEvent event) {
    if (_closed) return;
    _events.add(event);
    _trim();
    _flushTimer ??= Timer(const Duration(milliseconds: 250), () {
      _flushTimer = null;
      _flush();
    });
  }

  void _trim() {
    if (_events.length > 256) _events.removeRange(0, _events.length - 256);
  }

  List<DiagnosticEvent> snapshot() => List.unmodifiable(_events);

  void _flush() {
    final root = _root;
    if (root == null) return;
    _dirty = true;
    if (_flushing) return;
    _flushing = true;
    _writing = _drain(root);
  }

  Future<void> _drain(Directory root) async {
    while (_dirty) {
      _dirty = false;
      final records = _events.map(encodeDiagnosticEvent).toList();
      try {
        final bytes = utf8.encode(jsonEncode(records));
        if (bytes.length > diagnosticByteLimit) {
          throw const FormatException("Oversized diagnostic journal");
        }
        final pending = File(
          "${root.path}${Platform.pathSeparator}events.pending",
        );
        final target = "${root.path}${Platform.pathSeparator}events.json";
        if (await FileSystemEntity.type(root.path, followLinks: false) !=
                FileSystemEntityType.directory ||
            await FileSystemEntity.type(pending.path, followLinks: false) ==
                FileSystemEntityType.link ||
            await FileSystemEntity.type(target, followLinks: false) ==
                FileSystemEntityType.link) {
          throw const FileSystemException("Linked diagnostic storage");
        }
        await pending.writeAsBytes(bytes, flush: true);
        await pending.rename(target);
        _available = true;
      } on Object {
        _available = false;
      }
    }
    _flushing = false;
  }

  Future<void> close() => _closing ??= _close();

  Future<void> _close() async {
    _closed = true;
    _flushTimer?.cancel();
    _flushTimer = null;
    await _initializing?.timeout(const Duration(seconds: 1), onTimeout: () {});
    _flush();
    await _writing;
  }
}
