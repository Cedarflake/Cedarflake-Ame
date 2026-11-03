import "../../domain/ame_preferences.dart";

abstract interface class AmePreferenceStore {
  Future<AmePreferences> loadAmePreferences();

  Future<void> saveAmePreferences(AmePreferences preferences);
}
