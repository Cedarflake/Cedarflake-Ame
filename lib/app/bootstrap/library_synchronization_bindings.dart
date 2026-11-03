import "package:flutter_riverpod/misc.dart";

import "../../features/library/adapters/rust/rust_library_synchronization_gateway.dart";
import "../../features/library/application/library_synchronization.dart";
import "../../features/library/application/ports/library_synchronization.dart";
import "library_synchronization_lifecycle_owner.dart";

class LibrarySynchronizationBindings {
  LibrarySynchronizationBindings.production()
    : this(
        LibrarySynchronizationController.production(
          RustLibrarySynchronizationGateway.production(),
        ),
      );

  LibrarySynchronizationBindings(this.synchronization)
    : lifecycle = LibrarySynchronizationLifecycleOwner(synchronization);

  final LibrarySynchronization synchronization;
  final LibrarySynchronizationLifecycleOwner lifecycle;

  List<Override> get overrides => [
    librarySynchronizationProvider.overrideWithValue(synchronization),
  ];
}

final librarySynchronizationRequirements = <ProviderListenable<Object?>>[
  librarySynchronizationProvider,
];
