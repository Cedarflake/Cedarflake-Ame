import "package:flutter_riverpod/misc.dart";

import "../../features/library/adapters/platform/windows_file_operation_platform.dart";
import "../../features/library/adapters/rust/rust_library_file_operations.dart";
import "../../features/library/application/library_file_operations.dart";

List<Override> libraryFileBindings() => [
  libraryFileOperationsProvider.overrideWithValue(
    const RustLibraryFileOperations(WindowsFileOperationPlatform()),
  ),
];

final libraryFileRequirements = <ProviderListenable<Object?>>[
  libraryFileOperationsProvider,
];
