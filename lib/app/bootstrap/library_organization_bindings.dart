import "package:flutter_riverpod/misc.dart";

import "../../features/library/adapters/rust/rust_library_organization.dart";
import "../../features/library/adapters/rust/rust_library_organization_batch.dart";
import "../../features/library/adapters/rust/rust_library_organization_archive.dart";
import "../../features/library/adapters/rust/rust_library_organization_cleanup.dart";
import "../../features/library/adapters/rust/rust_library_organization_conflicts.dart";
import "../../features/library/adapters/rust/rust_library_review.dart";
import "../../features/library/adapters/rust/rust_library_sidebar_order.dart";
import "../../features/library/application/library_organization.dart";
import "../../features/library/application/library_organization_batch.dart";
import "../../features/library/application/library_organization_archive.dart";
import "../../features/library/application/library_organization_cleanup.dart";
import "../../features/library/application/library_organization_conflicts.dart";
import "../../features/library/application/library_review.dart";
import "../../features/library/application/library_sidebar_order.dart";

List<Override> libraryOrganizationBindings() => [
  libraryOrganizationProvider.overrideWithValue(
    const RustLibraryOrganization(),
  ),
  libraryOrganizationBatchesProvider.overrideWithValue(
    const RustLibraryOrganizationBatches(),
  ),
  libraryReviewProvider.overrideWithValue(const RustLibraryReview()),
  libraryOrganizationArchivesProvider.overrideWithValue(
    const RustLibraryOrganizationArchives(),
  ),
  libraryOrganizationCleanupProvider.overrideWithValue(
    const RustLibraryOrganizationCleanup(),
  ),
  libraryOrganizationConflictsProvider.overrideWithValue(
    const RustLibraryOrganizationConflicts(),
  ),
  librarySidebarOrderProvider.overrideWithValue(
    const RustLibrarySidebarOrder(),
  ),
];

final libraryOrganizationRequirements = <ProviderListenable<Object?>>[
  libraryOrganizationProvider,
  libraryOrganizationBatchesProvider,
  libraryReviewProvider,
  libraryOrganizationArchivesProvider,
  libraryOrganizationCleanupProvider,
  libraryOrganizationConflictsProvider,
  librarySidebarOrderProvider,
];
