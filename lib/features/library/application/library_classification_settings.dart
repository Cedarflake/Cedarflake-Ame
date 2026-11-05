import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_classification_configuration.dart";
import "../domain/library_model_installation.dart";
import "library_classification_configuration.dart";
import "library_controller.dart";
import "library_model_installation.dart";

final libraryClassificationSettingsProvider =
    FutureProvider.autoDispose<LibraryClassificationSettings?>((ref) {
      _watchClassificationChanges(ref);
      return ref.watch(libraryClassificationConfigurationProvider)?.load();
    });

final libraryClassificationNavigationProvider =
    FutureProvider.autoDispose<LibraryClassificationSettings?>((ref) {
      _watchClassificationChanges(ref);
      return ref
          .watch(libraryClassificationConfigurationProvider)
          ?.loadNavigation();
    });

void _watchClassificationChanges(Ref ref) {
  ref.watch(
    libraryModelInstallationProvider.select(
      (installation) =>
          installation?.stage == LibraryModelInstallationStage.enabled
          ? installation?.sequence
          : null,
    ),
  );
  ref.watch(libraryControllerProvider.select((state) => state.catalogRevision));
}

final libraryClassificationCategoriesProvider =
    Provider.autoDispose<AsyncValue<List<LibraryClassificationCategory>>>((
      ref,
    ) {
      final settings = ref.watch(libraryClassificationSettingsProvider);
      return settings.whenData((value) => value?.visibleCategories ?? const []);
    });
