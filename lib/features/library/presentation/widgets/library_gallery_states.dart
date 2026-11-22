import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_progress.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "../library_first_import_feedback.dart";
import "library_gallery_message.dart";

/// 图库中央的空白区域提示入口；按加载、恢复、首次添加及空结果选择对应文案。
/// 首次添加的标题和说明配对见 LibraryFirstImportFeedback。
class EmptyLibraryGallery extends StatelessWidget {
  const EmptyLibraryGallery({
    required this.state,
    required this.onImport,
    super.key,
  });

  final LibraryState state;
  final VoidCallback onImport;

  @override
  Widget build(BuildContext context) {
    if (state.catalogRevision == null &&
        (state.status == LibraryStatus.refreshing ||
            state.isRefreshingQuery ||
            state.isLoadingTimeline)) {
      return const GalleryLoadingState();
    }
    if (_isAwaitingPublishedDisplay) {
      if (state.primaryScanSnapshot.publication ==
          LibraryScanPublication.confirmationPending) {
        return _PendingLibraryGallery(
          "library-scan-confirmation-state",
          title: context.l10n.scanConfirmationPending,
          detail: context.l10n.scanConfirmationDetail,
        );
      }
      return _PendingLibraryGallery(
        "library-published-reload-state",
        title: context.l10n.galleryDisplayReloadPending,
        detail: context.l10n.galleryDisplayReloadDetail,
      );
    }
    if (_hasUnpublishedSource) {
      final feedback = LibraryFirstImportFeedback.fromState(
        context.l10n,
        state,
      );
      return _PendingLibraryGallery(
        "library-first-import-state",
        title: feedback.title,
        detail: feedback.detail,
      );
    }
    if (state.roots.isNotEmpty) {
      return NoGalleryResults(query: state.query);
    }
    return EmptyLibrary(state: state, onImport: onImport);
  }

  bool get _isAwaitingPublishedDisplay {
    final scan = state.primaryScanSnapshot;
    if (!scan.hasUnresolvedPublication) {
      return false;
    }
    final selectedRootId = state.query.rootId;
    if (selectedRootId == null) {
      return true;
    }
    return state.roots.any(
      (root) =>
          root.id == selectedRootId &&
          (root.path == scan.rootPath ||
              root.displayPath == scan.displayRootPath),
    );
  }

  bool get _hasUnpublishedSource {
    final selectedRootId = state.query.rootId;
    for (final root in state.roots) {
      if ((selectedRootId == null || root.id == selectedRootId) &&
          root.activeScanId == null) {
        return true;
      }
    }
    if (selectedRootId != null) {
      return false;
    }
    final scan = state.primaryScanSnapshot;
    if (scan.taskKind != LibraryTaskKind.import ||
        scan.rootPath == null ||
        !(scan.isScanning || scan.hasRetainedScan)) {
      return false;
    }
    // A newly admitted import can precede the first registered-root read.
    return !state.roots.any(
      (root) =>
          root.path == scan.rootPath ||
          root.displayPath == scan.displayRootPath,
    );
  }
}

// 图库中央｜首次添加或加载恢复时的目录图标、标题和说明。
class _PendingLibraryGallery extends StatelessWidget {
  const _PendingLibraryGallery(
    this.stateKey, {
    required this.title,
    required this.detail,
  });

  final String stateKey;
  final String title;
  final String? detail;

  @override
  Widget build(BuildContext context) {
    return Center(
      key: Key(stateKey),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(Symbols.folder_rounded, size: 48),
              const SizedBox(height: 16),
              Text(
                title,
                style: Theme.of(context).textTheme.titleLarge,
                textAlign: TextAlign.center,
              ),
              if (detail case final message?) ...[
                const SizedBox(height: 8),
                Text(message, textAlign: TextAlign.center),
              ],
            ],
          ),
        ),
      ),
    );
  }
}

/// Initial waiting feedback when the empty-state view is hosted independently.
class GalleryLoadingState extends StatelessWidget {
  const GalleryLoadingState({super.key});

  @override
  Widget build(BuildContext context) {
    return Center(
      key: Key("library-query-loading"),
      child: AmeLoadingProgress(
        semanticsLabel: context.l10n.loadingPhotosShort,
      ),
    );
  }
}

/// 图库中央｜搜索无结果和所选目录无图片时的两组提示。
class NoGalleryResults extends StatelessWidget {
  const NoGalleryResults({required this.query, super.key});

  final LibraryGalleryQuery query;

  @override
  Widget build(BuildContext context) {
    if (query.organizationQueue case final queue?) {
      return LibraryGalleryMessage(
        key: const Key("library-no-results"),
        icon: Symbols.checklist_rounded,
        title: switch (queue) {
          LibraryOrganizationQueue.pending => context.l10n.noPendingPhotos,
          LibraryOrganizationQueue.deferred => context.l10n.noDeferredPhotos,
          LibraryOrganizationQueue.ignored => context.l10n.noIgnoredPhotos,
        },
        detail: context.l10n.organizationEmptyHint,
      );
    }
    if (query.semanticResults != null || query.metadataFilter.isActive) {
      return LibraryGalleryMessage(
        key: const Key("library-no-results"),
        icon: Symbols.image_search_rounded,
        title: context.l10n.noSearchResults,
        detail: query.semanticResults != null
            ? context.l10n.contentSearchEmptyHint
            : context.l10n.filterSearchEmptyHint,
      );
    }
    final isSearch = query.searchText.isNotEmpty;
    final wholeCollection =
        !isSearch && query.rootId == null && query.organizationQueue == null;
    if (query.collection is LibrarySmartAlbumCollection && wholeCollection) {
      return LibraryGalleryMessage(
        key: const Key("library-no-results"),
        icon: Symbols.auto_awesome_rounded,
        title: context.l10n.noCategoryPhotos,
        detail: context.l10n.categoryEmptyHint,
      );
    }
    if (wholeCollection && query.collection is LibraryAlbumCollection) {
      return LibraryGalleryMessage(
        key: Key("library-no-results"),
        icon: Symbols.photo_album_rounded,
        title: context.l10n.noAlbumPhotos,
        detail: context.l10n.albumEmptyHint,
      );
    }
    if (wholeCollection && query.collection is LibraryFavoritesCollection) {
      return LibraryGalleryMessage(
        key: Key("library-no-results"),
        icon: Symbols.favorite_rounded,
        title: context.l10n.noFavoritePhotos,
        detail: context.l10n.favoritesEmptyHint,
      );
    }
    return LibraryGalleryMessage(
      key: const Key("library-no-results"),
      icon: isSearch ? Symbols.image_search_rounded : Symbols.photo_rounded,
      title: isSearch
          ? context.l10n.noSearchResults
          : context.l10n.noSourceResults,
      detail: isSearch
          ? context.l10n.noSearchResultsHint
          : context.l10n.noSourceResultsHint,
    );
  }
}

/// 图库中央｜没有已添加目录时的引导、导入按钮及暂停任务的补充提示。
class EmptyLibrary extends StatelessWidget {
  const EmptyLibrary({required this.state, required this.onImport, super.key});

  final LibraryState state;
  final VoidCallback onImport;

  @override
  Widget build(BuildContext context) {
    return Center(
      key: const Key("library-empty-state"),
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 520),
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(
                state.isProcessing && !state.isRefreshingQuery
                    ? Symbols.hourglass_top_rounded
                    : Symbols.add_photo_alternate_rounded,
                size: 56,
                color: Theme.of(context).colorScheme.primary,
              ),
              const SizedBox(height: 20),
              Text(
                context.l10n.emptyLibraryTitle,
                style: Theme.of(context).textTheme.headlineSmall,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 12),
              Text(context.l10n.emptyLibraryBody, textAlign: TextAlign.center),
              const SizedBox(height: 24),
              AbsorbPointer(
                absorbing: state.isRefreshingQuery,
                child: FilledButton.icon(
                  onPressed:
                      state.hasRetainedScan ||
                          (state.isBusy && !state.isRefreshingQuery)
                      ? null
                      : () {
                          if (!state.isBusy) onImport();
                        },
                  icon: const Icon(Symbols.create_new_folder_rounded),
                  label: Text(context.l10n.importPhotos),
                ),
              ),
              if (state.hasRetainedScan) ...[
                const SizedBox(height: 8),
                Text(context.l10n.resumeOrCancelPaused),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
