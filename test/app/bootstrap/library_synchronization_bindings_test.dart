import "package:cedarflake_ame/app/bootstrap/ame_provider_assembly.dart";
import "package:cedarflake_ame/app/bootstrap/library_synchronization_bindings.dart";
import "package:cedarflake_ame/app/window/ame_shutdown_coordinator.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/application/ports/library_synchronization.dart";
import "package:flutter_test/flutter_test.dart";

import "../../support/inert_library_synchronization.dart";

void main() {
  test(
    "provider and startup lifecycle share one synchronization service",
    () async {
      final synchronization = _Synchronization();
      final bindings = LibrarySynchronizationBindings(synchronization);
      final shutdown = AmeShutdownCoordinator();
      final assembly = AmeProviderAssembly(shutdown: shutdown);
      shutdown.register(bindings.lifecycle.close);
      final container = assembly.assemble(
        requirements: librarySynchronizationRequirements,
        overrides: bindings.overrides,
      );
      expect(
        container.read(librarySynchronizationProvider),
        same(synchronization),
      );
      bindings.lifecycle.startInBackground();
      bindings.lifecycle.startInBackground();
      expect(
        await bindings.lifecycle.startOperation,
        LibrarySynchronizationStartResult.started,
      );
      expect(synchronization.starts, 1);
      final close = shutdown.shutdown();
      await close;
      await bindings.lifecycle.close();
      expect(synchronization.disposals, 1);
      expect(
        () => container.read(librarySynchronizationProvider),
        throwsStateError,
      );
    },
  );

  test(
    "missing synchronization binding fails before application publication",
    () {
      expect(
        () => AmeProviderAssembly(shutdown: AmeShutdownCoordinator()).assemble(
          requirements: librarySynchronizationRequirements,
          overrides: [],
        ),
        throwsA(isA<Exception>()),
      );
    },
  );
}

class _Synchronization extends InertLibrarySynchronization {
  int starts = 0;
  int disposals = 0;

  @override
  Future<LibrarySynchronizationStartResult> start() async {
    starts += 1;
    return LibrarySynchronizationStartResult.started;
  }

  @override
  Future<void> dispose() async {
    disposals += 1;
  }
}
