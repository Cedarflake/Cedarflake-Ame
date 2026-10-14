import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "../library_first_import_feedback.dart";
import "../library_strings.dart";

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
    if (state.status == LibraryStatus.refreshing ||
        state.isRefreshingQuery ||
        state.isLoadingTimeline) {
      return const GalleryLoadingState();
    }
    if (_isAwaitingPublishedDisplay) {
      if (state.primaryScanSnapshot.publication ==
          LibraryScanPublication.confirmationPending) {
        return const _PendingLibraryGallery(
          "library-scan-confirmation-state",
          title: LibraryStrings.scanConfirmationPending,
          detail: LibraryStrings.scanConfirmationDetail,
        );
      }
      return const _PendingLibraryGallery(
        "library-published-reload-state",
        title: LibraryStrings.galleryDisplayReloadPending,
        detail: LibraryStrings.galleryDisplayReloadDetail,
      );
    }
    if (_hasUnpublishedSource) {
      final feedback = LibraryFirstImportFeedback.fromState(state);
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

class GalleryLoadingState extends StatelessWidget {
  const GalleryLoadingState({super.key});

  @override
  Widget build(BuildContext context) {
    return const Center(
      key: Key("library-query-loading"),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          CircularProgressIndicator(),
          SizedBox(height: 16),
          Text(LibraryStrings.loadingLibrary),
        ],
      ),
    );
  }
}

class NoGalleryResults extends StatelessWidget {
  const NoGalleryResults({required this.query, super.key});

  final LibraryGalleryQuery query;

  @override
  Widget build(BuildContext context) {
    final isSearch = query.searchText.isNotEmpty;
    return Center(
      key: const Key("library-no-results"),
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              isSearch ? Symbols.image_search_rounded : Symbols.photo_rounded,
              size: 48,
            ),
            const SizedBox(height: 16),
            Text(
              isSearch
                  ? LibraryStrings.noSearchResults
                  : LibraryStrings.noSourceResults,
              style: Theme.of(context).textTheme.titleLarge,
            ),
            const SizedBox(height: 8),
            Text(
              isSearch
                  ? LibraryStrings.noSearchResultsHint
                  : LibraryStrings.noSourceResultsHint,
              textAlign: TextAlign.center,
            ),
          ],
        ),
      ),
    );
  }
}

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
                state.isProcessing
                    ? Symbols.hourglass_top_rounded
                    : Symbols.add_photo_alternate_rounded,
                size: 56,
                color: Theme.of(context).colorScheme.primary,
              ),
              const SizedBox(height: 20),
              Text(
                LibraryStrings.emptyLibraryTitle,
                style: Theme.of(context).textTheme.headlineSmall,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 12),
              const Text(
                LibraryStrings.emptyLibraryBody,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 24),
              FilledButton.icon(
                onPressed: state.isBusy || state.hasRetainedScan
                    ? null
                    : onImport,
                icon: const Icon(Symbols.create_new_folder_rounded),
                label: const Text(LibraryStrings.import),
              ),
              if (state.hasRetainedScan) ...[
                const SizedBox(height: 8),
                const Text("请先继续或取消已暂停的任务"),
              ],
            ],
          ),
        ),
      ),
    );
  }
}
