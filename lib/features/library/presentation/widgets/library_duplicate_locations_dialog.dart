import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_controller.dart";
import "../../application/library_duplicate_results_controller.dart";
import "../../application/library_exact_duplicates.dart";
import "../../domain/library_exact_duplicates.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_location_tile.dart";

class LibraryDuplicateLocationsDialog extends ConsumerStatefulWidget {
  const LibraryDuplicateLocationsDialog({
    required this.group,
    required this.currentLocationId,
    super.key,
  });

  final LibraryDuplicateGroup group;
  final String currentLocationId;

  @override
  ConsumerState<LibraryDuplicateLocationsDialog> createState() =>
      _LibraryDuplicateLocationsDialogState();
}

class _LibraryDuplicateLocationsDialogState
    extends ConsumerState<LibraryDuplicateLocationsDialog> {
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
      (_, _) => unawaited(_results.catalogChanged()),
    );
  }

  @override
  void didUpdateWidget(covariant LibraryDuplicateLocationsDialog oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.group.digest != oldWidget.group.digest ||
        widget.group.byteLength != oldWidget.group.byteLength) {
      unawaited(_results.showLocations(widget.group));
    }
  }

  @override
  void dispose() {
    _results.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Dialog(
      constraints: const BoxConstraints(maxWidth: 560),
      insetPadding: const EdgeInsets.all(24),
      clipBehavior: Clip.antiAlias,
      child: SizedBox(
        key: const Key("duplicate-locations-dialog"),
        width: 560,
        height: 520,
        child: ListenableBuilder(
          listenable: _results,
          builder: (context, _) => Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 8, 8, 8),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        LibraryDuplicateStrings.viewLocations,
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                    ),
                    AmeTooltip(
                      message: LibraryDuplicateStrings.close,
                      child: IconButton(
                        key: const Key("duplicate-locations-close"),
                        autofocus: true,
                        onPressed: () => Navigator.of(context).pop(),
                        icon: const Icon(Symbols.close_rounded),
                      ),
                    ),
                  ],
                ),
              ),
              const Divider(height: 1),
              Expanded(child: _buildLocations()),
              if (_results.canPrevious || _results.canNext)
                Padding(
                  padding: const EdgeInsets.all(8),
                  child: Row(
                    mainAxisAlignment: MainAxisAlignment.end,
                    children: [
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
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildLocations() {
    if (_results.loading) {
      return const Center(child: CircularProgressIndicator());
    }
    if (_results.hasError) {
      return Center(
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
      );
    }
    if (_results.locations.locations.isEmpty) {
      return const Center(child: Text(LibraryDuplicateStrings.emptyLocations));
    }
    return AmeScrollMotion(
      child: ListView.builder(
        primary: false,
        itemCount: _results.locations.locations.length,
        itemBuilder: (_, index) {
          final location = _results.locations.locations[index];
          return LibraryDuplicateLocationTile(
            location: location,
            isCurrent: location.locationId == widget.currentLocationId,
            wrapPath: true,
          );
        },
      ),
    );
  }
}
