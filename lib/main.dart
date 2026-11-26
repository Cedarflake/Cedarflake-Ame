import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "app/ame_app.dart";
import "app/bootstrap/ame_application_mount.dart";
import "app/bootstrap/application_update_bindings.dart";
import "app/bootstrap/application_update_restart.dart";
import "app/bootstrap/ame_bootstrap_failure.dart";
import "app/bootstrap/ame_provider_assembly.dart";
import "app/bootstrap/ame_startup_orchestrator.dart";
import "app/bootstrap/library_classification_bindings.dart";
import "app/bootstrap/library_duplicate_bindings.dart";
import "app/bootstrap/library_file_bindings.dart";
import "app/bootstrap/library_organization_bindings.dart";
import "app/bootstrap/library_platform_bindings.dart";
import "app/bootstrap/library_read_bindings.dart";
import "app/bootstrap/library_scan_bindings.dart";
import "app/bootstrap/library_similarity_bindings.dart";
import "app/bootstrap/library_synchronization_bindings.dart";
import "app/bootstrap/library_synchronization_lifecycle_owner.dart";
import "app/bootstrap/dependency_licenses.dart";
import "app/bootstrap/diagnostics_bindings.dart";
import "app/bootstrap/rust_library_loader.dart";
import "app/bootstrap/settings_bindings.dart";
import "app/bootstrap/storage_bindings.dart";
import "app/bootstrap/workbench_presentation_bindings.dart";
import "app/window/ame_analysis_environment.dart";
import "app/window/ame_shutdown_coordinator.dart";
import "app/window/ame_window_actions.dart";
import "app/window/window_manager_actions.dart";
import "features/library/application/library_catalog.dart";
import "features/library/application/library_classification_lifecycle.dart";
import "features/library/application/library_controller.dart";
import "features/library/application/library_foreground_reads.dart";
import "features/library/application/library_file_operations.dart";
import "features/library/application/library_scan_shutdown.dart";
import "features/library/domain/library_models.dart";
import "features/library/domain/library_state.dart";

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  registerDependencyLicenses();
  final preferenceStores = AmePreferenceStores.production();
  final shutdownCoordinator = AmeShutdownCoordinator();
  final windowShutdown = AmeShutdownCoordinator();
  windowShutdown.register(shutdownCoordinator.shutdown);
  final amePreferences = await preferenceStores.application
      .loadAmePreferences();
  final viewPreferences = await preferenceStores.library
      .loadLibraryViewPreferences();
  final windowActions = await initializeAmeWindow(
    preferenceStores.window,
    windowShutdown,
  );
  final diagnostics = DiagnosticsBindings();
  windowShutdown.register(diagnostics.close);
  final applicationMount = AmeApplicationMount();
  final providerAssembly = AmeProviderAssembly(
    shutdown: shutdownCoordinator,
    unmountApplication: applicationMount.unmount,
  );

  LibrarySynchronizationLifecycleOwner? synchronizationLifecycle;
  LibraryClassificationLifecycle? classificationLifecycle;
  Future<void> Function()? recoverModelInstallation;
  ApplicationUpdateBindings? updates;
  late final Widget application;
  try {
    application = await acquireAmeApplication(
      shutdown: shutdownCoordinator,
      acquire: () async {
        await initializeRustLibrary();
        diagnostics.nativeReady();
        final updateBindings = ApplicationUpdateBindings(
          restarter: ApplicationUpdateRestart(windowActions.retirement),
        );
        updates = updateBindings;
        shutdownCoordinator.register(updateBindings.close);
        final reads = LibraryReadBindings.production();
        final query = LibraryGalleryQuery(
          sortKey: viewPreferences.sortKey,
          sortDirection: viewPreferences.sortDirection,
          duplicateMode: viewPreferences.duplicateMode,
        );
        final initialSnapshot = await reads.catalog.loadQuerySnapshot(
          maxItems: libraryCatalogWindow,
          query: query,
        );
        final initialState =
            LibraryState.fromSnapshot(
              initialSnapshot.snapshot,
              query: query,
            ).copyWith(
              timeline: initialSnapshot.timeline,
              layoutManifest: initialSnapshot.layoutManifest,
              organizationSummary: initialSnapshot.organizationSummary,
            );
        final synchronizationBindings =
            LibrarySynchronizationBindings.production();
        final synchronizationOwner = synchronizationBindings.lifecycle;
        synchronizationLifecycle = synchronizationOwner;
        final scanShutdownCoordinator = LibraryScanShutdownCoordinator();
        shutdownCoordinator.register(synchronizationOwner.close);
        shutdownCoordinator.register(scanShutdownCoordinator.suspend);
        final duplicates = LibraryDuplicateBindings.production();
        final exactAnalysisBindings = LibraryExactAnalysisBindings(
          service: duplicates.exactDuplicates,
          paused: false,
          foregroundReads: libraryForegroundReads,
        );
        shutdownCoordinator.register(exactAnalysisBindings.close);
        final similarAnalysisBindings =
            LibrarySimilarAnalysisBindings.production(
              foregroundReads: libraryForegroundReads,
            );
        shutdownCoordinator.register(similarAnalysisBindings.close);
        final classificationBindings = LibraryClassificationBindings(
          services: const LibraryClassificationServices.production(),
          foregroundReads: libraryForegroundReads,
        );
        final classificationOwner = classificationBindings.lifecycle;
        classificationLifecycle = classificationOwner;
        shutdownCoordinator.register(classificationOwner.close);
        final classificationEnvironment = AmeAnalysisEnvironment(
          onSuspendedChanged: (suspended) {
            classificationOwner.setSuspended(suspended);
            similarAnalysisBindings.scheduler.setSuspended(suspended);
          },
          onMemoryPressure: () {
            classificationOwner.memoryPressure();
            similarAnalysisBindings.scheduler.memoryPressure();
          },
          onError: (error, stack) {
            classificationOwner.reportEnvironmentError(error, stack);
            similarAnalysisBindings.scheduler.reportEnvironmentError(
              error,
              stack,
            );
          },
        );
        shutdownCoordinator.register(classificationEnvironment.dispose);
        unawaited(classificationEnvironment.start());
        shutdownCoordinator.register(
          classificationBindings.modelInstallation.close,
        );
        recoverModelInstallation =
            classificationBindings.recoverModelInstallation;
        final container = providerAssembly.assemble(
          requirements: [
            ...libraryPlatformRequirements,
            ...settingsRequirements,
            ...applicationUpdateRequirements,
            ...storageRequirements,
            ...workbenchPresentationRequirements,
            ...libraryReadRequirements,
            ...libraryScanRequirements,
            ...libraryDuplicateRequirements,
            ...libraryExactAnalysisRequirements,
            ...librarySimilarAnalysisRequirements,
            ...libraryOrganizationRequirements,
            ...libraryFileRequirements,
            ...libraryClassificationRequirements,
            ...librarySynchronizationRequirements,
          ],
          overrides: [
            ...diagnostics.overrides,
            ...updateBindings.overrides,
            ...libraryPlatformBindings(
              preferenceStore: preferenceStores.library,
              initialPreferences: viewPreferences,
            ),
            ...settingsBindings(
              preferenceStore: preferenceStores.application,
              initialPreferences: amePreferences,
            ),
            ...storageBindings(),
            ...workbenchPresentationBindings(),
            ...reads.overrides,
            ...libraryScanBindings(),
            ...duplicates.overrides,
            ...exactAnalysisBindings.overrides,
            ...similarAnalysisBindings.overrides,
            ...libraryOrganizationBindings(),
            ...libraryFileBindings(),
            ...synchronizationBindings.overrides,
            ...classificationBindings.overrides,
            ameWindowActionsProvider.overrideWithValue(windowActions),
            libraryScanShutdownCoordinatorProvider.overrideWithValue(
              scanShutdownCoordinator,
            ),
            initialLibraryStateProvider.overrideWithValue(initialState),
          ],
        );
        shutdownCoordinator.register(
          container.read(libraryFileActionsProvider.notifier).close,
        );
        updateBindings.attachNotifications(container);
        diagnostics.attach(container);
        return UncontrolledProviderScope(
          container: container,
          child: const AmeApp(),
        );
      },
    );
  } on Object catch (error) {
    diagnostics.startupFailed(error);
    synchronizationLifecycle = null;
    classificationLifecycle = null;
    recoverModelInstallation = null;
    updates = null;
    application = ProviderScope(
      overrides: [
        ...diagnostics.overrides,
        ameWindowActionsProvider.overrideWithValue(windowActions),
      ],
      child: AmeBootstrapFailure(error: error, preferences: amePreferences),
    );
  }
  await mountAmeApplicationAndRevealWindow(
    mountApplication: () => applicationMount.mount(application),
    startInBackground: () {
      synchronizationLifecycle?.startInBackground();
      classificationLifecycle?.startInBackground();
      final recover = recoverModelInstallation;
      if (recover != null) unawaited(recover());
    },
    revealWindowAfterFirstFrame: showAmeWindowAfterFirstFrame,
  );
  final updateBindings = updates;
  if (updateBindings != null) unawaited(updateBindings.start());
}
