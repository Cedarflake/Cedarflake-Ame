import "../../../app/presentation/ame_localizations.dart";
import "../domain/library_exact_duplicates.dart";

// Duplicates workspace: analysis, result groups and physical-location inspection.
abstract final class LibraryDuplicateStrings {
  static String selectGroup(
    AppLocalizations strings,
    String name, {
    required bool selected,
  }) => selected
      ? strings.deselectNamedGroup(name)
      : strings.selectNamedGroup(name);

  // Each location row is refreshed using metadata only; opening the location list never downloads files.
  static String? availability(
    AppLocalizations strings,
    LibraryDuplicateAvailability value,
  ) => switch (value) {
    LibraryDuplicateAvailability.local => null,
    LibraryDuplicateAvailability.cloudOnly => strings.cloudOnly,
    LibraryDuplicateAvailability.offline => strings.folderOffline,
    LibraryDuplicateAvailability.unavailable => strings.temporarilyInaccessible,
  };
}
