import "package:flutter_riverpod/misc.dart";

import "../../features/library/application/ports/library_view_preference_store.dart";
import "../../features/settings/adapters/platform/organization_archive_picker.dart";
import "../../features/settings/adapters/platform/shared_preferences_ame_store.dart";
import "../../features/settings/application/ame_preferences.dart";
import "../../features/settings/application/organization_archive_picker.dart";
import "../../features/settings/application/ports/ame_preference_store.dart";
import "../../features/settings/application/ports/organization_archive_picker.dart";
import "../../features/settings/domain/ame_preferences.dart";
import "../window/ame_window_placement.dart";

class AmePreferenceStores {
  const AmePreferenceStores({
    required this.application,
    required this.library,
    required this.window,
  });

  factory AmePreferenceStores.production() {
    final store = SharedPreferencesAmeStore();
    return AmePreferenceStores(
      application: store,
      library: store,
      window: store,
    );
  }

  final AmePreferenceStore application;
  final LibraryViewPreferenceStore library;
  final AmeWindowPreferenceStore window;
}

List<Override> settingsBindings({
  required AmePreferenceStore preferenceStore,
  required AmePreferences initialPreferences,
  OrganizationArchivePicker archivePicker =
      const PlatformOrganizationArchivePicker(),
}) => [
  amePreferenceStoreProvider.overrideWithValue(preferenceStore),
  initialAmePreferencesProvider.overrideWithValue(initialPreferences),
  organizationArchivePickerProvider.overrideWithValue(archivePicker),
];

final settingsRequirements = <ProviderListenable<Object?>>[
  amePreferenceStoreProvider,
  organizationArchivePickerProvider,
];
