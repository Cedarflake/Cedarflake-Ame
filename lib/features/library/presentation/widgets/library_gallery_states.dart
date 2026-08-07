import "package:flutter/material.dart";

import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "../library_strings.dart";

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
          Text(LibraryStrings.updatingLibrary),
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
              isSearch ? Icons.image_search_outlined : Icons.photo_outlined,
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
                    ? Icons.hourglass_top
                    : Icons.add_photo_alternate_outlined,
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
                onPressed: state.isBusy ? null : onImport,
                icon: const Icon(Icons.create_new_folder_outlined),
                label: const Text(LibraryStrings.import),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
