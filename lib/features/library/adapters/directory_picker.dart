import "package:file_selector/file_selector.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

abstract interface class DirectoryPicker {
  Future<String?> pickDirectory();
}

class FileSelectorDirectoryPicker implements DirectoryPicker {
  const FileSelectorDirectoryPicker();

  @override
  Future<String?> pickDirectory() {
    return getDirectoryPath(confirmButtonText: "导入文件夹");
  }
}

final directoryPickerProvider = Provider<DirectoryPicker>((ref) {
  return const FileSelectorDirectoryPicker();
});
