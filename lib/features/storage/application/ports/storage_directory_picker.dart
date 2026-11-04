abstract interface class StorageDirectoryPicker {
  Future<String?> pick({required String currentPath});
}
