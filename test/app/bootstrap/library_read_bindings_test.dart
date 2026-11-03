import "dart:io";

import "package:cedarflake_ame/app/bootstrap/ame_provider_assembly.dart";
import "package:cedarflake_ame/app/bootstrap/library_duplicate_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_organization_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_read_bindings.dart";
import "package:cedarflake_ame/app/bootstrap/library_scan_bindings.dart";
import "package:cedarflake_ame/app/window/ame_shutdown_coordinator.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_catalog.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_organization.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_organization_archive.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_organization_batch.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_organization_cleanup.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_organization_conflicts.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_previewer.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_review.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_scanner.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_sidebar_order.dart";
import "package:cedarflake_ame/features/library/adapters/rust/rust_library_source_reader.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_duplicate_review.dart";
import "package:cedarflake_ame/features/library/application/library_foreground_reads.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_organization.dart";
import "package:cedarflake_ame/features/library/application/library_organization_archive.dart";
import "package:cedarflake_ame/features/library/application/library_organization_batch.dart";
import "package:cedarflake_ame/features/library/application/library_organization_cleanup.dart";
import "package:cedarflake_ame/features/library/application/library_organization_conflicts.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_review.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_sidebar_order.dart";
import "package:cedarflake_ame/features/library/application/library_source_reader.dart";
import "package:cedarflake_ame/features/library/presentation/library_preview_images.dart";
import "package:flutter/painting.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  final requirements = [
    ...libraryReadRequirements,
    ...libraryScanRequirements,
    ...libraryDuplicateRequirements,
    ...libraryOrganizationRequirements,
  ];

  test(
    "production read ports share catalog and foreground authority",
    () async {
      final foreground = LibraryForegroundReads();
      addTearDown(foreground.dispose);
      final reads = LibraryReadBindings.production(foregroundReads: foreground);
      final shutdown = AmeShutdownCoordinator();
      final container = AmeProviderAssembly(shutdown: shutdown).assemble(
        requirements: requirements,
        overrides: [
          ...reads.overrides,
          ...libraryScanBindings(),
          ...LibraryDuplicateBindings.production().overrides,
          ...libraryOrganizationBindings(),
        ],
      );
      addTearDown(shutdown.shutdown);
      expect(container.read(libraryCatalogProvider), same(reads.catalog));
      expect(container.read(libraryCatalogProvider), isA<RustLibraryCatalog>());
      expect(container.read(libraryFolderCatalogProvider), same(reads.catalog));
      final previewImages = container.read(libraryPreviewImagesProvider);
      const previewPath = "test-cache/preview.jpg";
      final expectedImage = FileImage(File(previewPath));
      expect(previewImages(previewPath), expectedImage);
      expect(
        ResizeImage.resizeIfNeeded(128, null, previewImages(previewPath)),
        ResizeImage.resizeIfNeeded(128, null, expectedImage),
      );
      expect(
        container.read(libraryGalleryLayoutManifestCatalogProvider),
        isA<RustLibraryGalleryLayoutManifestCatalog>(),
      );
      expect(
        container.read(libraryPreviewerProvider),
        isA<RustLibraryPreviewer>().having(
          (reader) => reader.foregroundReads,
          "foreground",
          same(foreground),
        ),
      );
      expect(
        container.read(librarySourceReaderProvider),
        isA<RustLibrarySourceReader>().having(
          (reader) => reader.foregroundReads,
          "foreground",
          same(foreground),
        ),
      );
      expect(container.read(libraryScannerProvider), isA<RustLibraryScanner>());
      expect(
        container.read(libraryDuplicateReviewProvider),
        isA<RustLibraryDuplicateReview>(),
      );
      expect(
        container.read(libraryOrganizationProvider),
        isA<RustLibraryOrganization>(),
      );
      expect(
        container.read(libraryOrganizationBatchesProvider),
        isA<RustLibraryOrganizationBatches>(),
      );
      expect(container.read(libraryReviewProvider), isA<RustLibraryReview>());
      expect(
        container.read(libraryOrganizationArchivesProvider),
        isA<RustLibraryOrganizationArchives>(),
      );
      expect(
        container.read(libraryOrganizationCleanupProvider),
        isA<RustLibraryOrganizationCleanup>(),
      );
      expect(
        container.read(libraryOrganizationConflictsProvider),
        isA<RustLibraryOrganizationConflicts>(),
      );
      expect(
        container.read(librarySidebarOrderProvider),
        isA<RustLibrarySidebarOrder>(),
      );
    },
  );

  test(
    "every omitted read or shared-mapper binding fails before publication",
    () {
      final overrides = [
        ...LibraryReadBindings.production().overrides,
        ...libraryScanBindings(),
        ...LibraryDuplicateBindings.production().overrides,
        ...libraryOrganizationBindings(),
      ];
      for (final omitted in requirements) {
        expect(
          () =>
              AmeProviderAssembly(shutdown: AmeShutdownCoordinator()).assemble(
                requirements: requirements,
                overrides: overrides
                    .where((value) => !identical(value.origin, omitted))
                    .toList(),
              ),
          throwsA(isA<Exception>()),
        );
      }
    },
  );
}
