import "package:flutter_riverpod/misc.dart";

import "../../features/library/adapters/platform/file_selector_directory_picker.dart";
import "../../features/library/adapters/platform/windows_library_platform_actions.dart";
import "../../features/library/application/directory_picker.dart";
import "../../features/library/application/library_platform_actions.dart";
import "../../features/library/application/library_view_preferences.dart";
import "../../features/library/application/ports/directory_picker.dart";
import "../../features/library/application/ports/library_platform_actions.dart";
import "../../features/library/application/ports/library_view_preference_store.dart";
import "../../features/library/domain/library_view_preferences.dart";

List<Override> libraryPlatformBindings({
  required LibraryViewPreferenceStore preferenceStore,
  required LibraryViewPreferences initialPreferences,
  DirectoryPicker directoryPicker = const FileSelectorDirectoryPicker(),
  LibraryPlatformActions platformActions =
      const WindowsLibraryPlatformActions(),
}) => [
  directoryPickerProvider.overrideWithValue(directoryPicker),
  libraryPlatformActionsProvider.overrideWithValue(platformActions),
  libraryViewPreferenceStoreProvider.overrideWithValue(preferenceStore),
  initialLibraryViewPreferencesProvider.overrideWithValue(initialPreferences),
];

final libraryPlatformRequirements = <ProviderListenable<Object?>>[
  directoryPickerProvider,
  libraryPlatformActionsProvider,
  libraryViewPreferenceStoreProvider,
];
