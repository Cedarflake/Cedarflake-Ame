import "package:cedarflake_ame/app/bootstrap/ame_provider_assembly.dart";
import "package:cedarflake_ame/app/bootstrap/library_classification_bindings.dart";
import "package:cedarflake_ame/app/window/ame_shutdown_coordinator.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_classification.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_classification_review.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_model_installation.dart";
import "package:cedarflake_ame/features/library/application/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/library_classification_lifecycle.dart";
import "package:cedarflake_ame/features/library/application/library_classification_review.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/application/library_model_installation.dart";
import "package:cedarflake_ame/features/library/application/ports/library_classification.dart";
import "package:cedarflake_ame/features/library/application/ports/library_classification_configuration.dart";
import "package:cedarflake_ame/features/library/application/ports/library_model_installation.dart";
import "package:cedarflake_ame/features/library/domain/library_classification_configuration.dart";
import "package:flutter/foundation.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "production bindings share runtime identity and retire before disposal",
    () async {
      const services = LibraryClassificationServices.production();
      final fixture = _Assembly(services);
      final runtime = fixture.runtime;
      expect(fixture.foreground.isObserved, isTrue);
      final container = fixture.assemble();
      final child = ProviderContainer(parent: container);

      expect(services.classification, isA<RustLibraryClassification>());
      expect(
        services.configuration,
        isA<RustLibraryClassificationConfiguration>(),
      );
      expect(services.installation, isA<RustLibraryModelInstallation>());
      expect(
        container.read(libraryClassificationReviewServiceProvider),
        isA<RustLibraryClassificationReview>(),
      );
      expect(
        child.read(libraryClassificationLifecycleProvider),
        same(runtime.lifecycle),
      );
      expect(
        child.read(libraryClassificationConfigurationProvider),
        same(runtime.configuration),
      );
      expect(
        child.read(libraryModelInstallationProvider),
        same(runtime.modelInstallation),
      );
      expect(runtime.modelInstallation.busy, isFalse);

      child.dispose();
      await fixture.shutdown.shutdown();
      expect(fixture.foreground.isObserved, isFalse);
      expect(
        () => ChangeNotifier.debugAssertNotDisposed(runtime.lifecycle),
        throwsFlutterError,
      );
      expect(
        () => ChangeNotifier.debugAssertNotDisposed(runtime.modelInstallation),
        throwsFlutterError,
      );
      fixture.foreground.dispose();
    },
  );

  test(
    "every omitted classification binding fails and releases acquisition",
    () async {
      for (final omitted in libraryClassificationRequirements) {
        final fixture = _Assembly(
          const LibraryClassificationServices.production(),
        );
        await expectLater(
          acquireAmeApplication(
            shutdown: fixture.shutdown,
            acquire: () async => fixture.assembly.assemble(
              requirements: libraryClassificationRequirements,
              overrides: fixture.runtime.overrides
                  .where((value) => !identical(value.origin, omitted))
                  .toList(),
            ),
          ),
          throwsA(isA<Exception>()),
        );
        expect(fixture.foreground.isObserved, isFalse);
        expect(fixture.runtime.modelInstallation.canStart, isFalse);
        fixture.foreground.dispose();
      }
    },
  );

  test(
    "startup without installation neither opens analysis nor authorizes download",
    () async {
      final classification = _Classification();
      final configuration = _Configuration();
      final installation = _Installation();
      final fixture = _Assembly(
        LibraryClassificationServices(
          classification: classification,
          configuration: configuration,
          review: const RustLibraryClassificationReview(),
          installation: installation,
        ),
      );
      fixture.assemble();
      expect(configuration.loads, 0);
      expect(installation.recoveries, 0);
      fixture.runtime.lifecycle.startInBackground();
      fixture.runtime.lifecycle.startInBackground();
      await fixture.runtime.lifecycle.waitUntilOpened();
      expect(configuration.loads, 1);
      expect(classification.opens, 0);

      expect(
        await fixture.runtime.configuration.loadNavigation(),
        same(configuration.settings),
      );
      expect(configuration.navigationLoads, 1);
      await fixture.runtime.recoverModelInstallation();
      expect(installation.recoveries, 1);
      expect(installation.creations, 0);
      expect(fixture.runtime.modelInstallation.error, isNull);
      await fixture.shutdown.shutdown();
      expect(fixture.foreground.isObserved, isFalse);
      fixture.foreground.dispose();
    },
  );
}

class _Assembly {
  _Assembly(LibraryClassificationServices services) {
    assembly = AmeProviderAssembly(shutdown: shutdown);
    runtime = LibraryClassificationBindings(
      services: services,
      foregroundReads: foreground,
    );
    shutdown.register(runtime.lifecycle.close);
    shutdown.register(runtime.modelInstallation.close);
  }

  final foreground = _Foreground();
  final shutdown = AmeShutdownCoordinator();
  late final AmeProviderAssembly assembly;
  late final LibraryClassificationBindings runtime;

  ProviderContainer assemble() => assembly.assemble(
    requirements: libraryClassificationRequirements,
    overrides: runtime.overrides,
  );
}

class _Foreground extends LibraryForegroundReads {
  bool get isObserved => hasListeners;
}

class _Classification implements LibraryClassification {
  int opens = 0;
  @override
  Future<LibraryClassificationSession?> open() async {
    opens++;
    return null;
  }
}

class _Configuration implements LibraryClassificationConfigurationService {
  @override
  Future<LibraryClassificationAcceleration> loadAcceleration() async =>
      const LibraryClassificationAcceleration(gpuEnabled: true);

  @override
  Future<void> saveAcceleration({required bool gpuEnabled}) async {
    throw UnsupportedError("Acceleration is outside this fixture");
  }

  int loads = 0;
  int navigationLoads = 0;
  final settings = LibraryClassificationSettings(
    revision: BigInt.one,
    dispatchRevision: BigInt.one,
    mode: LibraryClassificationMode.disabled,
    categories: const [],
    runId: null,
    installationSelected: false,
  );

  @override
  Future<LibraryClassificationSettings> load() async {
    loads++;
    return settings;
  }

  @override
  Future<LibraryClassificationSettings> loadNavigation() async {
    navigationLoads++;
    return settings;
  }

  @override
  Future<List<LibraryClassificationCategory>> initialCategories() async =>
      throw StateError("Unexpected category activation");
  @override
  Future<LibraryClassificationSettings> save({
    required String operationId,
    required LibraryClassificationSettings observed,
    required List<LibraryClassificationCategory> categories,
    required LibraryClassificationConfigurationAction action,
  }) async => throw StateError("Unexpected activation");
  @override
  Future<LibraryClassificationSettings> changeControl({
    required BigInt expectedRevision,
    required LibraryClassificationControl action,
  }) async => throw StateError("Unexpected control change");
}

class _Installation implements LibraryModelInstallationService {
  int recoveries = 0;
  int creations = 0;
  @override
  Future<LibraryModelInstallationSession?> recover() async {
    recoveries++;
    return null;
  }

  @override
  LibraryModelInstallationSession create() {
    creations++;
    throw StateError("Unexpected download authorization");
  }
}
