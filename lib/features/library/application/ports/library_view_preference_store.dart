import "../../domain/library_view_preferences.dart";

abstract interface class LibraryViewPreferenceStore {
  Future<LibraryViewPreferences> loadLibraryViewPreferences();

  Future<void> saveLibraryViewPreferences(LibraryViewPreferences preferences);
}
