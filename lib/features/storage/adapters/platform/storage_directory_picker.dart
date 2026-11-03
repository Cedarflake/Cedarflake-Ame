import "dart:io";

import "package:file_selector/file_selector.dart";
import "package:flutter/foundation.dart";

import "../../application/ports/storage_directory_picker.dart";

class PlatformStorageDirectoryPicker implements StorageDirectoryPicker {
  const PlatformStorageDirectoryPicker() : _selectDirectory = _pickDirectory;

  @visibleForTesting
  const PlatformStorageDirectoryPicker.withSelector(this._selectDirectory);

  final Future<String?> Function(String initialDirectory) _selectDirectory;

  @override
  Future<String?> pick({required String currentPath}) =>
      _selectDirectory(Directory(currentPath).parent.path);

  static Future<String?> _pickDirectory(String initialDirectory) {
    return getDirectoryPath(
      initialDirectory: initialDirectory,
      confirmButtonText: "选择此目录",
    );
  }
}
