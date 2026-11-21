import "../../../app/presentation/ame_localizations.dart";

String galleryDateLabel(
  AppLocalizations strings,
  String? key, {
  bool monthOnly = false,
}) {
  if (key == null) return strings.unknownCaptureDate;
  final dayKey = monthOnly ? "$key-01" : key;
  final date = DateTime.tryParse(dayKey);
  if (date == null || !date.toIso8601String().startsWith("${dayKey}T")) {
    return key;
  }
  return monthOnly ? strings.galleryMonth(date) : strings.galleryDay(date);
}
