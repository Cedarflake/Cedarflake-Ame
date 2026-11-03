import "package:cedarflake_ame/app/bootstrap/library_platform_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_read_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_scan_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_duplicate_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_organization_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/settings_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/storage_bindings.dart";
import "package:cedarflake_ame/features/library/application/ports/library_view_preference_store.dart";
import "package:cedarflake_ame/features/library/domain/library_view_preferences.dart";
import "package:cedarflake_ame/features/settings/application/ports/ame_preference_store.dart";
import "package:cedarflake_ame/features/settings/domain/ame_preferences.dart";
import "package:flutter_riverpod/misc.dart";

List<Override> applicationTestOverrides([List<Override> overrides = const []]) {
  final preferences = MemoryPreferenceStore();
  final production = [
    ...libraryPlatformBindings(
      preferenceStore: preferences,
      initialPreferences: const LibraryViewPreferences(),
    ),
    ...settingsBindings(
      preferenceStore: preferences,
      initialPreferences: const AmePreferences(),
    ),
    ...storageBindings(),
    ...LibraryReadBindings.production().overrides,
    ...libraryScanBindings(),
    ...libraryDuplicateBindings(),
    ...libraryOrganizationBindings(),
  ];
  return {
    for (final override in production) override.origin: override,
    for (final override in overrides) override.origin: override,
  }.values.toList();
}

class MemoryPreferenceStore
    implements AmePreferenceStore, LibraryViewPreferenceStore {
  AmePreferences _application = const AmePreferences();
  LibraryViewPreferences _library = const LibraryViewPreferences();

  @override
  Future<AmePreferences> loadAmePreferences() async => _application;

  @override
  Future<void> saveAmePreferences(AmePreferences preferences) async {
    _application = preferences;
  }

  @override
  Future<LibraryViewPreferences> loadLibraryViewPreferences() async => _library;

  @override
  Future<void> saveLibraryViewPreferences(
    LibraryViewPreferences preferences,
  ) async {
    _library = preferences;
  }
}
