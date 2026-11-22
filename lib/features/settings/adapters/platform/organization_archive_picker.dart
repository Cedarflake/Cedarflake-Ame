import "package:file_selector/file_selector.dart";

import "../../application/ports/organization_archive_picker.dart";

class PlatformOrganizationArchivePicker implements OrganizationArchivePicker {
  const PlatformOrganizationArchivePicker(this._labels);
  final ({String type, String export, String choose}) Function() _labels;
  static XTypeGroup _type(String label) =>
      XTypeGroup(label: label, extensions: ["ame-backup"]);

  @override
  Future<String?> destination() async {
    final labels = _labels();
    final date = DateTime.now().toIso8601String().substring(0, 10);
    return (await getSaveLocation(
      acceptedTypeGroups: [_type(labels.type)],
      suggestedName: "Ame-$date.ame-backup",
      confirmButtonText: labels.export,
    ))?.path;
  }

  @override
  Future<String?> backup() async {
    final labels = _labels();
    return (await openFile(
      acceptedTypeGroups: [_type(labels.type)],
      confirmButtonText: labels.choose,
    ))?.path;
  }
}
