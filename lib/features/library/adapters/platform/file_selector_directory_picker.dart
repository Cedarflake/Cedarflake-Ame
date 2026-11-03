import "package:file_selector/file_selector.dart";

import "../../application/ports/directory_picker.dart";

class FileSelectorDirectoryPicker implements DirectoryPicker {
  const FileSelectorDirectoryPicker();

  @override
  Future<String?> pickDirectory() {
    return getDirectoryPath(confirmButtonText: "导入目录");
  }
}
