import "dart:io";

import "package:file_selector/file_selector.dart";
import "package:flutter/foundation.dart";

import "../../application/ports/storage_directory_picker.dart";

class PlatformStorageDirectoryPicker implements StorageDirectoryPicker {
  PlatformStorageDirectoryPicker(String Function() confirmLabel)
    : _selectDirectory = ((initialDirectory) => getDirectoryPath(
        initialDirectory: initialDirectory,
        confirmButtonText: confirmLabel(),
      ));

  @visibleForTesting
  const PlatformStorageDirectoryPicker.withSelector(this._selectDirectory);

  final Future<String?> Function(String initialDirectory) _selectDirectory;

  @override
  Future<String?> pick({required String currentPath}) =>
      _selectDirectory(Directory(currentPath).parent.path);
}
