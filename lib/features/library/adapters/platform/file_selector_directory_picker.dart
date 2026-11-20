import "package:file_selector/file_selector.dart";

import "../../application/ports/directory_picker.dart";

class FileSelectorDirectoryPicker implements DirectoryPicker {
  const FileSelectorDirectoryPicker(this._confirmLabel);
  final String Function() _confirmLabel;

  @override
  Future<String?> pickDirectory() {
    return getDirectoryPath(confirmButtonText: _confirmLabel());
  }
}
