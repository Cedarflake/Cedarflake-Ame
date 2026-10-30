import "../../../src/rust/api/classification.dart" as session;
import "../../../src/rust/api/classification_settings.dart" as native;
import "../../../src/rust/domain/classification/work.dart" as work;
import "../../../src/rust/domain/classification_configuration.dart" as contract;
import "../application/library_classification_configuration.dart";

class RustLibraryClassificationConfiguration
    implements LibraryClassificationConfigurationService {
  const RustLibraryClassificationConfiguration();

  @override
  Future<LibraryClassificationSettings> load() async =>
      _settings(await native.loadClassificationSettings());

  @override
  Future<List<LibraryClassificationCategory>> initialCategories() async =>
      List.unmodifiable(
        (await native.initialClassificationCategories()).map(_category),
      );

  @override
  Future<void> selectInstallation({
    required String models,
    required String runtime,
  }) => session.selectClassificationInstallation(
    models: models,
    runtime: runtime,
  );

  @override
  Future<LibraryClassificationSettings> save({
    required String operationId,
    required LibraryClassificationSettings observed,
    required List<LibraryClassificationCategory> categories,
    required LibraryClassificationConfigurationAction action,
  }) async => _settings(
    await native.saveClassificationSettings(
      operationId: operationId,
      expectedCatalogRevision: observed.revision,
      expectedDispatchRevision: observed.dispatchRevision,
      categories: categories
          .map(
            (category) => contract.ClassificationCategoryInput(
              id: category.id,
              label: category.label,
              descriptions: category.descriptions,
            ),
          )
          .toList(growable: false),
      action: switch (action) {
        LibraryClassificationConfigurationAction.save =>
          contract.ClassificationConfigurationAction.save,
        LibraryClassificationConfigurationAction.enable =>
          contract.ClassificationConfigurationAction.enable,
      },
    ),
  );

  @override
  Future<LibraryClassificationSettings> changeControl({
    required BigInt expectedRevision,
    required LibraryClassificationControl action,
  }) async => _settings(
    await native.changeClassificationControl(
      expectedRevision: expectedRevision,
      action: switch (action) {
        LibraryClassificationControl.pause =>
          contract.ClassificationControlAction.pause,
        LibraryClassificationControl.resume =>
          contract.ClassificationControlAction.continue_,
        LibraryClassificationControl.disable =>
          contract.ClassificationControlAction.disable,
        LibraryClassificationControl.stop =>
          contract.ClassificationControlAction.stop,
      },
    ),
  );
}

LibraryClassificationCategory _category(
  contract.ClassificationCategoryInput category,
) => LibraryClassificationCategory(
  id: category.id,
  label: category.label,
  descriptions: List.unmodifiable(category.descriptions),
);

LibraryClassificationSettings _settings(
  contract.ClassificationSettings settings,
) => LibraryClassificationSettings(
  revision: settings.revision,
  dispatchRevision: settings.dispatchRevision,
  mode: switch (settings.mode) {
    work.ClassificationDispatchMode.disabled =>
      LibraryClassificationMode.disabled,
    work.ClassificationDispatchMode.automatic =>
      LibraryClassificationMode.automatic,
    work.ClassificationDispatchMode.paused => LibraryClassificationMode.paused,
    work.ClassificationDispatchMode.stopped =>
      LibraryClassificationMode.stopped,
  },
  categories: List.unmodifiable(settings.categories.map(_category)),
  runId: settings.runId,
  installationSelected: settings.installationSelected,
);
