import "package:flutter_riverpod/misc.dart";

import "../../features/library/adapters/platform/library_preview_image.dart";
import "../../features/library/adapters/rust/rust_library_catalog.dart";
import "../../features/library/adapters/rust/rust_library_layout_manifest_catalog.dart";
import "../../features/library/adapters/rust/rust_library_previewer.dart";
import "../../features/library/adapters/rust/rust_library_source_reader.dart";
import "../../features/library/application/library_catalog.dart";
import "../../features/library/application/library_foreground_reads.dart";
import "../../features/library/application/library_layout_manifest_catalog.dart";
import "../../features/library/application/library_previewer.dart";
import "../../features/library/application/library_source_reader.dart";
import "../../features/library/application/ports/library_catalog.dart";
import "../../features/library/application/ports/library_foreground_read_activity.dart";
import "../../features/library/application/ports/library_previewer.dart";
import "../../features/library/application/ports/library_source_reader.dart";
import "../../features/library/presentation/library_preview_images.dart";

class LibraryReadBindings {
  LibraryReadBindings.production({
    LibraryForegroundReadActivity? foregroundReads,
  }) : _catalog = const RustLibraryCatalog(),
       _previewer = RustLibraryPreviewer(
         foregroundReads: foregroundReads ?? libraryForegroundReads,
       ),
       _sourceReader = RustLibrarySourceReader(
         foregroundReads: foregroundReads ?? libraryForegroundReads,
       );

  final RustLibraryCatalog _catalog;
  final LibraryPreviewer _previewer;
  final LibrarySourceReader _sourceReader;

  LibraryCatalog get catalog => _catalog;

  List<Override> get overrides => [
    libraryCatalogProvider.overrideWithValue(_catalog),
    libraryGroupMemberCatalogProvider.overrideWithValue(_catalog),
    libraryFolderCatalogProvider.overrideWithValue(_catalog),
    libraryGalleryLayoutManifestCatalogProvider.overrideWithValue(
      const RustLibraryGalleryLayoutManifestCatalog(),
    ),
    libraryPreviewerProvider.overrideWithValue(_previewer),
    librarySourceReaderProvider.overrideWithValue(_sourceReader),
    libraryPreviewImagesProvider.overrideWithValue(createLibraryPreviewImage),
  ];
}

final libraryReadRequirements = <ProviderListenable<Object?>>[
  libraryCatalogProvider,
  libraryGroupMemberCatalogProvider,
  libraryFolderCatalogProvider,
  libraryGalleryLayoutManifestCatalogProvider,
  libraryPreviewerProvider,
  librarySourceReaderProvider,
  libraryPreviewImagesProvider,
];
