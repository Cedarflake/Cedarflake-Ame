import "package:cedarflake_ame/app/bootstrap/library_classification_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_duplicate_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_organization_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_platform_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_read_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_scan_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_similarity_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_synchronization_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/settings_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/storage_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/workbench_presentation_bindings.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_lifecycle.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_exact_analysis_scheduler.dart";
import "package:cedarflake_ame/features/library/application/library_exact_duplicates.dart";
import "package:cedarflake_ame/features/library/application/library_gallery_duplicates.dart";
import "package:cedarflake_ame/features/library/application/library_model_installation.dart";
import "package:cedarflake_ame/features/library/application/library_organization_conflicts.dart";
import "package:cedarflake_ame/features/library/application/library_similar_analysis_controller.dart";
import "package:cedarflake_ame/features/library/application/library_similar_analysis_scheduler.dart";
import "package:cedarflake_ame/features/library/application/ports/library_view_preference_store.dart";
import "package:cedarflake_ame/features/library/domain/library_view_preferences.dart";
import "package:cedarflake_ame/features/settings/application/ports/ame_preference_store.dart";
import "package:cedarflake_ame/features/settings/domain/ame_preferences.dart";
import "package:flutter_riverpod/misc.dart";

import "inert_library_synchronization.dart";

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
    ...workbenchPresentationBindings(),
    ...LibraryReadBindings.production().overrides,
    ...libraryScanBindings(),
    ...LibrarySynchronizationBindings(InertLibrarySynchronization()).overrides,
    ...LibraryDuplicateBindings.production().overrides,
    libraryExactAnalysisControllerProvider.overrideWith((ref) {
      final analysis = LibraryExactAnalysisController(
        ref.watch(libraryExactDuplicatesProvider),
      );
      ref.onDispose(analysis.dispose);
      return analysis;
    }),
    libraryExactAnalysisSchedulerProvider.overrideWithValue(null),
    librarySimilarAnalysisProvider.overrideWithValue(
      LibrarySimilarAnalysisBindings.productionService,
    ),
    librarySimilarAnalysisControllerProvider.overrideWith((ref) {
      final analysis = LibrarySimilarAnalysisController(
        ref.watch(librarySimilarAnalysisProvider),
      );
      ref.onDispose(analysis.dispose);
      return analysis;
    }),
    librarySimilarAnalysisSchedulerProvider.overrideWithValue(null),
    libraryGalleryDuplicatesProvider.overrideWithValue(null),
    ...const LibraryClassificationServices.production().overrides,
    libraryClassificationConfigurationProvider.overrideWithValue(null),
    libraryClassificationLifecycleProvider.overrideWith((ref) => null),
    libraryModelInstallationProvider.overrideWith((ref) => null),
    ...libraryOrganizationBindings(),
    libraryOrganizationConflictsProvider.overrideWithValue(null),
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
