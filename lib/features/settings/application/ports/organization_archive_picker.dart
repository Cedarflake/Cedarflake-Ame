abstract interface class OrganizationArchivePicker {
  Future<String?> destination();
  Future<String?> backup();
}
