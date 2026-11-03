import "package:flutter_riverpod/misc.dart";

import "../../features/library/adapters/rust/rust_library_organization.dart";
import "../../features/library/adapters/rust/rust_library_organization_batch.dart";
import "../../features/library/adapters/rust/rust_library_review.dart";
import "../../features/library/application/library_organization.dart";
import "../../features/library/application/library_organization_batch.dart";
import "../../features/library/application/library_review.dart";

List<Override> libraryOrganizationBindings() => [
  libraryOrganizationProvider.overrideWithValue(
    const RustLibraryOrganization(),
  ),
  libraryOrganizationBatchesProvider.overrideWithValue(
    const RustLibraryOrganizationBatches(),
  ),
  libraryReviewProvider.overrideWithValue(const RustLibraryReview()),
];

final libraryOrganizationRequirements = <ProviderListenable<Object?>>[
  libraryOrganizationProvider,
  libraryOrganizationBatchesProvider,
  libraryReviewProvider,
];
