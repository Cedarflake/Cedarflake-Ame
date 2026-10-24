import "package:file_selector/file_selector.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

abstract interface class OrganizationArchivePicker {
  Future<String?> destination();
  Future<String?> backup();
}

final organizationArchivePickerProvider = Provider<OrganizationArchivePicker>(
  (ref) => const PlatformOrganizationArchivePicker(),
);

class PlatformOrganizationArchivePicker implements OrganizationArchivePicker {
  const PlatformOrganizationArchivePicker();
  static const _type = XTypeGroup(
    label: "Ame 整理记录",
    extensions: ["ame-backup"],
  );

  @override
  Future<String?> destination() async {
    final date = DateTime.now().toIso8601String().substring(0, 10);
    return (await getSaveLocation(
      acceptedTypeGroups: [_type],
      suggestedName: "Ame-$date.ame-backup",
      confirmButtonText: "导出",
    ))?.path;
  }

  @override
  Future<String?> backup() async => (await openFile(
    acceptedTypeGroups: [_type],
    confirmButtonText: "选择备份",
  ))?.path;
}
