import "package:flutter_riverpod/flutter_riverpod.dart";

import "library_classification_configuration.dart";
import "library_controller.dart";
import "library_model_installation.dart";

final libraryClassificationSettingsProvider =
    FutureProvider.autoDispose<LibraryClassificationSettings?>((ref) {
      ref.watch(
        libraryModelInstallationProvider.select(
          (installation) =>
              installation?.stage == LibraryModelInstallationStage.enabled
              ? installation?.sequence
              : null,
        ),
      );
      ref.watch(
        libraryControllerProvider.select((state) => state.catalogRevision),
      );
      return ref.watch(libraryClassificationConfigurationProvider)?.load();
    });

final libraryClassificationCategoriesProvider =
    Provider.autoDispose<AsyncValue<List<LibraryClassificationCategory>>>((
      ref,
    ) {
      final settings = ref.watch(libraryClassificationSettingsProvider);
      return settings.whenData((value) => value?.categories ?? const []);
    });
