import "package:flutter_riverpod/misc.dart";

import "../../features/library/adapters/rust/rust_library_scanner.dart";
import "../../features/library/application/library_scanner.dart";

List<Override> libraryScanBindings() => [
  libraryScannerProvider.overrideWithValue(const RustLibraryScanner()),
];

final libraryScanRequirements = <ProviderListenable<Object?>>[
  libraryScannerProvider,
];
