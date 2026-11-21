import "../../../app/presentation/ame_localizations.dart";
import "../domain/library_metadata_filter.dart";

abstract final class LibraryFilterStrings {
  static String dateFieldName(
    AppLocalizations strings,
    LibraryDateField field,
  ) => switch (field) {
    LibraryDateField.capture => strings.captureDate,
    LibraryDateField.created => strings.createdDate,
    LibraryDateField.modified => strings.modifiedDate,
  };
}
