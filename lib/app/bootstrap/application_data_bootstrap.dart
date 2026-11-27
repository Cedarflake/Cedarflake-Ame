import "dart:io";

import "package:shared_preferences/shared_preferences.dart";
import "package:shared_preferences_windows/shared_preferences_windows.dart";

import "../../src/rust/api/storage.dart" as storage;
import "../window/ame_window_placement.dart";
import "rust_library_loader.dart";

class ApplicationDataBootstrap {
  ApplicationDataBootstrap._(this.directory, this._failure);

  final String? directory;
  final ({Object error, StackTrace stack})? _failure;

  static Future<ApplicationDataBootstrap> prepare() async {
    String? directory;
    try {
      await initializeRustLibrary();
      directory = await storage.applicationPreferencesDirectory();
      return ApplicationDataBootstrap._(directory, null);
    } on Object catch (error, stack) {
      return ApplicationDataBootstrap._(directory, (
        error: error,
        stack: stack,
      ));
    }
  }

  SharedPreferencesAsync preferences() {
    final path = directory;
    if (path == null || !Platform.isWindows) return SharedPreferencesAsync();
    return SharedPreferencesAsync(
      options: SharedPreferencesWindowsOptions(
        fileName: "$path${Platform.pathSeparator}shared_preferences",
      ),
    );
  }

  Future<Directory> diagnosticDirectory() async {
    requireReady();
    final path = directory;
    if (path == null) {
      throw StateError("Application data directory is unavailable");
    }
    return Directory("$path${Platform.pathSeparator}diagnostics");
  }

  AmeWindowPreferenceStore windowPreferences(AmeWindowPreferenceStore store) =>
      _failure == null ? store : _FailureWindowPreferences(store);

  void requireReady() {
    final failure = _failure;
    if (failure != null) {
      Error.throwWithStackTrace(failure.error, failure.stack);
    }
  }
}

class _FailureWindowPreferences implements AmeWindowPreferenceStore {
  const _FailureWindowPreferences(this._store);

  final AmeWindowPreferenceStore _store;

  @override
  Future<AmeWindowPlacement?> loadWindowPlacement() =>
      _store.loadWindowPlacement();

  @override
  Future<void> saveWindowPlacement(AmeWindowPlacement placement) async {
    // Keep failed storage initialization from writing preferences to a fallback location.
  }
}
