import "dart:async";

import "package:cedarflake_ame/app/bootstrap/ame_application_mount.dart";
import "package:cedarflake_ame/app/bootstrap/ame_provider_assembly.dart";
import "package:cedarflake_ame/app/bootstrap/library_platform_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/settings_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/storage_bindings.dart";
import "package:cedarflake_ame/app/window/ame_shutdown_coordinator.dart";
import "package:cedarflake_ame/features/library/adapters/platform/file_selector_directory_picker.dart";
import "package:cedarflake_ame/features/library/adapters/platform/windows_library_platform_actions.dart";
import "package:cedarflake_ame/features/library/application/directory_picker.dart";
import "package:cedarflake_ame/features/library/application/library_platform_actions.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/library_view_preferences.dart";
import "package:cedarflake_ame/features/settings/adapters/platform/organization_archive_picker.dart";
import "package:cedarflake_ame/features/settings/application/ame_preferences.dart";
import "package:cedarflake_ame/features/settings/application/organization_archive_picker.dart";
import "package:cedarflake_ame/features/settings/domain/ame_preferences.dart";
import "package:cedarflake_ame/features/storage/adapters/platform/storage_directory_picker.dart";
import "package:cedarflake_ame/features/storage/adapters/rust/rust_storage_settings_gateway.dart";
import "package:cedarflake_ame/features/storage/application/storage_settings.dart";
import "package:flutter/widgets.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";
import "package:shared_preferences_platform_interface/in_memory_shared_preferences_async.dart";
import "package:shared_preferences_platform_interface/shared_preferences_async_platform_interface.dart";

void main() {
  final requirements = [
    ...libraryPlatformRequirements,
    ...settingsRequirements,
    ...storageRequirements,
  ];

  setUp(() {
    SharedPreferencesAsyncPlatform.instance =
        InMemorySharedPreferencesAsync.empty();
  });

  test(
    "production roster shares one store and binds every platform port",
    () async {
      final stores = AmePreferenceStores.production();
      expect(stores.application, same(stores.library));
      expect(stores.application, same(stores.window));
      const initial = AmePreferences(sidebarWidth: 320);
      const view = LibraryViewPreferences(
        layoutShape: GalleryLayoutShape.square,
      );
      final shutdown = AmeShutdownCoordinator();
      final container = AmeProviderAssembly(shutdown: shutdown).assemble(
        requirements: requirements,
        overrides: [
          ...libraryPlatformBindings(
            preferenceStore: stores.library,
            initialPreferences: view,
          ),
          ...settingsBindings(
            preferenceStore: stores.application,
            initialPreferences: initial,
          ),
          ...storageBindings(),
        ],
      );
      expect(
        container.read(directoryPickerProvider),
        isA<FileSelectorDirectoryPicker>(),
      );
      expect(
        container.read(libraryPlatformActionsProvider),
        isA<WindowsLibraryPlatformActions>(),
      );
      expect(
        container.read(organizationArchivePickerProvider),
        isA<PlatformOrganizationArchivePicker>(),
      );
      expect(
        container.read(storageDirectoryPickerProvider),
        isA<PlatformStorageDirectoryPicker>(),
      );
      expect(
        container.read(storageSettingsGatewayProvider),
        isA<RustStorageSettingsGateway>(),
      );
      expect(
        container.read(amePreferenceStoreProvider),
        same(stores.application),
      );
      expect(
        container.read(libraryViewPreferenceStoreProvider),
        same(stores.library),
      );
      expect(container.read(initialLibraryViewPreferencesProvider), same(view));
      expect(container.read(amePreferencesControllerProvider), same(initial));
      await container
          .read(amePreferencesControllerProvider.notifier)
          .update(initial.copyWith(sidebarWidth: 340));
      expect(
        (await AmePreferenceStores.production().application
                .loadAmePreferences())
            .sidebarWidth,
        340,
      );
      await shutdown.shutdown();
      expect(
        () => container.read(amePreferenceStoreProvider),
        throwsStateError,
      );
    },
  );

  testWidgets(
    "background work and mounted consumers retire before container disposal",
    (tester) async {
      final shutdown = AmeShutdownCoordinator();
      final applicationMount = AmeApplicationMount();
      await applicationMount.unmount();
      final assembly = AmeProviderAssembly(
        shutdown: shutdown,
        unmountApplication: applicationMount.unmount,
      );
      final events = <String>[];
      final resource = Provider((ref) {
        ref.onDispose(() => events.add("provider"));
        return Object();
      });
      late final ProviderContainer container;
      Object? observedDuringRetirement;
      Object? observedDuringUnmount;
      shutdown.register(() {
        observedDuringRetirement = container.read(resource);
        events.add("background");
      });
      container = await acquireAmeApplication(
        shutdown: shutdown,
        acquire: () async =>
            assembly.assemble(overrides: [], requirements: [resource]),
      );
      final original = container.read(resource);
      applicationMount.mount(
        UncontrolledProviderScope(
          container: container,
          child: _RetirementProbe(
            onDispose: () {
              observedDuringUnmount = container.read(resource);
              events.add("presentation");
            },
            child: Consumer(
              builder: (_, ref, _) {
                ref.watch(resource);
                return const SizedBox.shrink();
              },
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(
        () => assembly.assemble(overrides: [], requirements: []),
        throwsStateError,
      );
      final closing = shutdown.shutdown();
      await tester.pumpAndSettle();
      await closing;
      expect(observedDuringRetirement, same(original));
      expect(observedDuringUnmount, same(original));
      expect(events, ["background", "presentation", "provider"]);
      expect(() => container.read(resource), throwsStateError);
      expect(
        () => assembly.assemble(overrides: [], requirements: []),
        throwsStateError,
      );
      await shutdown.shutdown();
      expect(events, ["background", "presentation", "provider"]);
    },
  );

  test("each omitted production port fails during assembly", () {
    final stores = AmePreferenceStores.production();
    final overrides = [
      ...libraryPlatformBindings(
        preferenceStore: stores.library,
        initialPreferences: const LibraryViewPreferences(),
      ),
      ...settingsBindings(
        preferenceStore: stores.application,
        initialPreferences: const AmePreferences(),
      ),
      ...storageBindings(),
    ];
    for (final omitted in requirements) {
      expect(
        () => AmeProviderAssembly(shutdown: AmeShutdownCoordinator()).assemble(
          requirements: requirements,
          overrides: overrides
              .where((override) => !identical(override.origin, omitted))
              .toList(),
        ),
        throwsA(isA<Exception>()),
      );
    }
  });

  test("failed assembly disposes providers already acquired", () {
    var releases = 0;
    final resource = Provider((ref) {
      ref.onDispose(() => releases += 1);
      return Object();
    });
    expect(
      () => AmeProviderAssembly(shutdown: AmeShutdownCoordinator()).assemble(
        overrides: [],
        requirements: [resource, directoryPickerProvider],
      ),
      throwsA(isA<Exception>()),
    );
    expect(releases, 1);
  });

  test(
    "startup failure joins acquired resources before returning the error",
    () async {
      final shutdown = AmeShutdownCoordinator();
      final retirement = Completer<void>();
      final events = <String>[];
      final failure = StateError("later acquisition failed");
      final result = acquireAmeApplication<void>(
        shutdown: shutdown,
        acquire: () async {
          shutdown.register(() => events.add("first"));
          shutdown.register(() async {
            await retirement.future;
            events.add("second");
          });
          throw failure;
        },
      );
      final expectation = expectLater(result, throwsA(same(failure)));
      await Future<void>.delayed(Duration.zero);
      expect(events, isEmpty);
      retirement.complete();
      await expectation;
      expect(events, ["second", "first"]);
      await shutdown.shutdown();
      expect(events, ["second", "first"]);
    },
  );

  test(
    "cleanup failure retains both the startup and retirement causes",
    () async {
      final shutdown = AmeShutdownCoordinator();
      final failure = StateError("assembly");
      final cleanup = StateError("retirement");
      var released = false;
      await expectLater(
        acquireAmeApplication<void>(
          shutdown: shutdown,
          acquire: () async {
            shutdown.register(() => released = true);
            shutdown.register(() => throw cleanup);
            throw failure;
          },
        ),
        throwsA(
          isA<AmeBootstrapCleanupFailure>()
              .having((error) => error.cause, "cause", same(failure))
              .having((error) => error.cleanupError, "cleanup", same(cleanup)),
        ),
      );
      expect(released, isTrue);
    },
  );
}

class _RetirementProbe extends StatefulWidget {
  const _RetirementProbe({required this.onDispose, required this.child});

  final VoidCallback onDispose;
  final Widget child;

  @override
  State<_RetirementProbe> createState() => _RetirementProbeState();
}

class _RetirementProbeState extends State<_RetirementProbe> {
  @override
  void dispose() {
    widget.onDispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
