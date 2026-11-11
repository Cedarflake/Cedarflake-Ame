import "package:flutter_riverpod/flutter_riverpod.dart";

import "../domain/library_classification_configuration.dart";
import "../domain/library_model_installation.dart";
import "library_classification_configuration.dart";
import "library_classification_navigation_reader.dart";
import "library_controller.dart";
import "library_foreground_reads.dart";
import "library_model_installation.dart";

final libraryClassificationSettingsProvider =
    FutureProvider.autoDispose<LibraryClassificationSettings?>((ref) {
      _watchClassificationChanges(ref);
      return ref.watch(libraryClassificationConfigurationProvider)?.load();
    });

final libraryClassificationNavigationProvider =
    StreamProvider<LibraryClassificationSettings?>((ref) {
      final configuration = ref.watch(
        libraryClassificationConfigurationProvider,
      );
      if (configuration == null) return Stream.value(null);
      final reader = LibraryClassificationNavigationReader(
        () => libraryForegroundReads.run(configuration.loadNavigation),
      );
      ref.onDispose(reader.close);
      ref.listen(
        libraryControllerProvider.select((state) => state.catalogRevision),
        (_, _) => reader.refresh(),
      );
      ref.listen(
        libraryModelInstallationProvider.select(
          (installation) =>
              installation?.stage == LibraryModelInstallationStage.enabled
              ? installation?.sequence
              : null,
        ),
        (_, _) => reader.refresh(),
      );
      reader.refresh();
      return reader.changes;
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
