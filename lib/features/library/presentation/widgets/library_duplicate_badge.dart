import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_controller.dart";
import "../../application/library_duplicate_results_controller.dart";
import "../../application/library_exact_duplicates.dart";
import "../../application/library_gallery_duplicates.dart";
import "../../domain/library_exact_duplicates.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_location_tile.dart";
import "library_gallery_card_metrics.dart";

class LibraryDuplicateBadge extends ConsumerWidget {
  const LibraryDuplicateBadge({required this.locationId, super.key});
  final String locationId;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final groups = ref.watch(libraryGalleryDuplicateAnnotationsProvider);
    final group = groups.asData?.value[locationId];
    if (group == null) {
      return const SizedBox.shrink();
    }
    return AmeTooltip(
      message:
          "${LibraryDuplicateStrings.viewLocations} · ${LibraryDuplicateStrings.locations(group.locationCount)}",
      child: TextButton.icon(
        key: ValueKey("duplicate-badge-$locationId"),
        style: TextButton.styleFrom(
          backgroundColor: Theme.of(
            context,
          ).colorScheme.surface.withValues(alpha: 0.92),
          foregroundColor: Theme.of(context).colorScheme.onSurface,
          minimumSize: const Size(40, 36),
          padding: const EdgeInsets.symmetric(
            horizontal: LibraryGalleryCardMetrics.duplicateHorizontalPadding,
          ),
          textStyle: LibraryGalleryCardMetrics.badgeTextStyleOf(context),
        ),
        onPressed: () => unawaited(
          showDialog<void>(
            context: context,
            builder: (_) => _DuplicateLocationsDialog(group: group),
          ),
        ),
        icon: const Icon(
          Symbols.photo_library_rounded,
          size: LibraryGalleryCardMetrics.duplicateIconExtent,
        ),
        label: Text("${group.locationCount}"),
      ),
    );
  }
}

class _DuplicateLocationsDialog extends ConsumerStatefulWidget {
  const _DuplicateLocationsDialog({required this.group});
  final LibraryDuplicateGroup group;

  @override
  ConsumerState<_DuplicateLocationsDialog> createState() =>
      _DuplicateLocationsDialogState();
}

class _DuplicateLocationsDialogState
    extends ConsumerState<_DuplicateLocationsDialog> {
  late final LibraryDuplicateResultsController _results;

  @override
  void initState() {
    super.initState();
    _results = LibraryDuplicateResultsController(
      ref.read(libraryExactDuplicatesProvider),
    );
    unawaited(_results.showLocations(widget.group));
    ref.listenManual(
      libraryControllerProvider.select((state) => state.catalogRevision),
      (_, _) {
        unawaited(_results.catalogChanged());
      },
    );
  }

  @override
  void dispose() {
    _results.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: _results,
    builder: (context, _) => AlertDialog(
      title: const Text(LibraryDuplicateStrings.viewLocations),
      content: SizedBox(
        width: 640,
        height: 360,
        child: switch ((_results.loading, _results.hasError)) {
          (true, _) => const Center(child: CircularProgressIndicator()),
          (_, true) => Center(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Text(LibraryDuplicateStrings.loadFailed),
                TextButton(
                  onPressed: _results.refresh,
                  child: const Text(LibraryDuplicateStrings.retry),
                ),
              ],
            ),
          ),
          _ when _results.locations.locations.isEmpty => const Center(
            child: Text(LibraryDuplicateStrings.emptyLocations),
          ),
          _ => AmeScrollMotion(
            child: ListView.builder(
              itemCount: _results.locations.locations.length,
              itemBuilder: (_, index) => LibraryDuplicateLocationTile(
                location: _results.locations.locations[index],
              ),
            ),
          ),
        },
      ),
      actions: [
        if (_results.canPrevious || _results.canNext) ...[
          TextButton(
            onPressed: !_results.loading && _results.canPrevious
                ? _results.previous
                : null,
            child: const Text(LibraryDuplicateStrings.previous),
          ),
          TextButton(
            onPressed: !_results.loading && _results.canNext
                ? _results.next
                : null,
            child: const Text(LibraryDuplicateStrings.next),
          ),
        ],
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text(LibraryDuplicateStrings.close),
        ),
      ],
    ),
  );
}
