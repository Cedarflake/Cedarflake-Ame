import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "app/ame_app.dart";
import "app/bootstrap/ame_bootstrap_failure.dart";
import "app/bootstrap/ame_provider_assembly.dart";
import "app/bootstrap/library_platform_bindings.dart";
import "app/bootstrap/library_read_bindings.dart";
import "app/bootstrap/library_scan_bindings.dart";
import "app/bootstrap/library_duplicate_bindings.dart";
import "app/bootstrap/library_organization_bindings.dart";
import "app/bootstrap/settings_bindings.dart";
import "app/bootstrap/storage_bindings.dart";
import "app/bootstrap/ame_startup_orchestrator.dart";
import "app/bootstrap/library_synchronization_lifecycle_owner.dart";
import "app/bootstrap/rust_library_loader.dart";
import "app/window/ame_window_actions.dart";
import "app/window/ame_analysis_environment.dart";
import "app/window/ame_shutdown_coordinator.dart";
import "app/window/window_manager_actions.dart";
import "features/library/application/library_catalog.dart";
import "features/library/adapters/rust_library_classification.dart";
import "features/library/adapters/rust_library_classification_configuration.dart";
import "features/library/adapters/rust_library_model_installation.dart";
import "features/library/application/library_classification_configuration.dart";
import "features/library/application/library_automatic_classification.dart";
import "features/library/application/library_classification_lifecycle.dart";
import "features/library/application/library_controller.dart";
import "features/library/application/library_exact_analysis_controller.dart";
import "features/library/application/library_exact_analysis_scheduler.dart";
import "features/library/application/library_gallery_duplicates.dart";
import "features/library/application/library_organization_conflicts.dart";
import "features/library/application/library_exact_duplicates.dart";
import "features/library/application/library_foreground_reads.dart";
import "features/library/application/library_model_installation.dart";
import "features/library/application/library_model_activation.dart";
import "features/library/application/library_scan_shutdown.dart";
import "features/library/application/library_synchronization.dart";
import "features/library/domain/library_models.dart";
import "features/library/domain/library_state.dart";

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final preferenceStores = AmePreferenceStores.production();
  final shutdownCoordinator = AmeShutdownCoordinator();
  final amePreferences = await preferenceStores.application
      .loadAmePreferences();
  final viewPreferences = await preferenceStores.library
      .loadLibraryViewPreferences();
  final windowActions = await initializeAmeWindow(
    preferenceStores.window,
    shutdownCoordinator,
  );
  final providerAssembly = AmeProviderAssembly(shutdown: shutdownCoordinator);

  LibrarySynchronizationLifecycleOwner? synchronizationLifecycle;
  LibraryClassificationLifecycle? classificationLifecycle;
  Future<void> Function()? recoverModelInstallation;
  late final Widget application;
  try {
    application = await acquireAmeApplication(
      shutdown: shutdownCoordinator,
      acquire: () async {
        await initializeRustLibrary();
        final reads = LibraryReadBindings.production();
        final query = LibraryGalleryQuery(
          sortKey: viewPreferences.sortKey,
          sortDirection: viewPreferences.sortDirection,
          duplicateMode: viewPreferences.duplicateMode,
        );
        final snapshot = await reads.catalog.load(
          maxItems: libraryCatalogWindow,
          query: query,
        );
        final initialState = LibraryState.fromSnapshot(snapshot, query: query);
        final synchronization = RustLibrarySynchronization.production();
        final synchronizationOwner = LibrarySynchronizationLifecycleOwner(
          synchronization,
        );
        synchronizationLifecycle = synchronizationOwner;
        final scanShutdownCoordinator = LibraryScanShutdownCoordinator();
        shutdownCoordinator.register(synchronizationOwner.close);
        shutdownCoordinator.register(scanShutdownCoordinator.suspend);
        final exactAnalysis = LibraryExactAnalysisController(
          const RustLibraryExactDuplicates(),
        );
        final exactScheduler = LibraryExactAnalysisScheduler(
          exactAnalysis,
          paused: amePreferences.exactAnalysisPaused,
          foregroundReads: libraryForegroundReads,
        );
        shutdownCoordinator.register(exactScheduler.close);
        final classificationOwner = LibraryClassificationLifecycle(
          const LibraryAutomaticClassification(
            RustLibraryClassification(),
            RustLibraryClassificationConfiguration(),
          ),
          foregroundReads: libraryForegroundReads,
        );
        classificationLifecycle = classificationOwner;
        shutdownCoordinator.register(classificationOwner.close);
        final classificationEnvironment = AmeAnalysisEnvironment(
          onSuspendedChanged: classificationOwner.setSuspended,
          onMemoryPressure: classificationOwner.memoryPressure,
          onError: classificationOwner.reportEnvironmentError,
        );
        shutdownCoordinator.register(classificationEnvironment.dispose);
        unawaited(classificationEnvironment.start());
        final classificationConfiguration = LibraryClassificationConfiguration(
          const RustLibraryClassificationConfiguration(),
          classificationOwner,
        );
        final modelInstallation = LibraryModelInstallation(
          const RustLibraryModelInstallation(),
        );
        shutdownCoordinator.register(modelInstallation.close);
        recoverModelInstallation = () => modelInstallation.recover(
          activateLibraryModel(classificationConfiguration),
        );
        final container = providerAssembly.assemble(
          requirements: [
            ...libraryPlatformRequirements,
            ...settingsRequirements,
            ...storageRequirements,
            ...libraryReadRequirements,
            ...libraryScanRequirements,
            ...libraryDuplicateRequirements,
            ...libraryOrganizationRequirements,
          ],
          overrides: [
            ...libraryPlatformBindings(
              preferenceStore: preferenceStores.library,
              initialPreferences: viewPreferences,
            ),
            ...settingsBindings(
              preferenceStore: preferenceStores.application,
              initialPreferences: amePreferences,
            ),
            ...storageBindings(),
            ...reads.overrides,
            ...libraryScanBindings(),
            ...libraryDuplicateBindings(),
            ...libraryOrganizationBindings(),
            libraryModelInstallationProvider.overrideWith(
              (ref) => modelInstallation,
            ),
            libraryClassificationConfigurationProvider.overrideWithValue(
              classificationConfiguration,
            ),
            libraryClassificationLifecycleProvider.overrideWith(
              (ref) => classificationOwner,
            ),
            libraryOrganizationConflictsProvider.overrideWithValue(
              const RustLibraryOrganizationConflicts(),
            ),
            libraryGalleryDuplicatesProvider.overrideWithValue(
              const RustLibraryGalleryDuplicates(),
            ),
            libraryExactAnalysisControllerProvider.overrideWithValue(
              exactAnalysis,
            ),
            libraryExactAnalysisSchedulerProvider.overrideWithValue(
              exactScheduler,
            ),
            ameWindowActionsProvider.overrideWithValue(windowActions),
            libraryScanShutdownCoordinatorProvider.overrideWithValue(
              scanShutdownCoordinator,
            ),
            librarySynchronizationProvider.overrideWithValue(synchronization),
            initialLibraryStateProvider.overrideWithValue(initialState),
          ],
        );
        return UncontrolledProviderScope(
          container: container,
          child: const AmeApp(),
        );
      },
    );
  } on Object catch (error) {
    synchronizationLifecycle = null;
    classificationLifecycle = null;
    recoverModelInstallation = null;
    application = ProviderScope(
      overrides: [ameWindowActionsProvider.overrideWithValue(windowActions)],
      child: AmeBootstrapFailure(error: error, preferences: amePreferences),
    );
  }
  await mountAmeApplicationAndRevealWindow(
    mountApplication: () => runApp(application),
    startInBackground: () {
      synchronizationLifecycle?.startInBackground();
      classificationLifecycle?.startInBackground();
      final recover = recoverModelInstallation;
      if (recover != null) unawaited(recover());
    },
    revealWindowAfterFirstFrame: showAmeWindowAfterFirstFrame,
  );
}
