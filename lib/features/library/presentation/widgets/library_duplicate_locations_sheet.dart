import "dart:async";
import "dart:math" as math;

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_controller.dart";
import "../../application/library_duplicate_review_window.dart";
import "../../application/library_exact_duplicates.dart";
import "../../domain/library_exact_duplicates.dart";
import "../library_duplicate_strings.dart";
import "library_duplicate_location_tile.dart";
import "library_duplicate_update_status.dart";

class LibraryDuplicateLocationsSheet extends ConsumerStatefulWidget {
  const LibraryDuplicateLocationsSheet({
    required this.group,
    required this.currentLocationId,
    super.key,
  });

  final LibraryDuplicateGroup group;
  final String currentLocationId;

  @override
  ConsumerState<LibraryDuplicateLocationsSheet> createState() =>
      _LibraryDuplicateLocationsSheetState();
}

class _LibraryDuplicateLocationsSheetState
    extends ConsumerState<LibraryDuplicateLocationsSheet> {
  late LibraryDuplicateReviewWindow<LibraryDuplicateLocation, String>
  _locations;
  int? _firstBuilt;
  int _endBuilt = 0;
  int _pendingCount = 0;

  @override
  void initState() {
    super.initState();
    _createWindow();
    ref.listenManual(
      libraryControllerProvider.select((state) => state.catalogRevision),
      (_, _) => unawaited(_locations.revalidate()),
    );
  }

  void _createWindow() {
    final service = ref.read(libraryExactDuplicatesProvider);
    final group = widget.group;
    _pendingCount = group.pendingLocationCount;
    _locations = LibraryDuplicateReviewWindow(
      (after) async {
        final page = await service.loadLocations(group, after);
        return _LocationSlice(page, group);
      },
      onAdmitted: (slice) {
        if (slice is _LocationSlice) _pendingCount = slice.pendingCount;
      },
    )..total = group.locationCount;
    unawaited(_locations.ensureRange(0, 1));
  }

  @override
  void didUpdateWidget(covariant LibraryDuplicateLocationsSheet oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.group.digest != oldWidget.group.digest ||
        widget.group.byteLength != oldWidget.group.byteLength) {
      _locations.dispose();
      _createWindow();
    }
  }

  // Variable-height paths let the framework's lazy list report its demand.
  void _requestBuiltRange(int index) {
    final first = _firstBuilt;
    _firstBuilt = first == null ? index : math.min(first, index);
    _endBuilt = math.max(_endBuilt, index + 1);
    if (first != null) return;
    final window = _locations;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      final start = _firstBuilt;
      final end = _endBuilt;
      _firstBuilt = null;
      _endBuilt = 0;
      if (mounted && identical(window, _locations) && start != null) {
        unawaited(_locations.ensureRange(start, end));
      }
    });
  }

  @override
  void dispose() {
    _locations.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => SafeArea(
    top: false,
    child: SizedBox(
      key: const Key("duplicate-locations-sheet"),
      height: MediaQuery.sizeOf(context).height * 0.72,
      child: ListenableBuilder(
        listenable: _locations,
        builder: (context, _) => Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 8, 24, 16),
              child: LibraryDuplicateUpdateStatus(
                hasPending: _pendingCount > 0,
                builder: (context, label) => Wrap(
                  alignment: WrapAlignment.spaceBetween,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  spacing: 16,
                  runSpacing: 4,
                  children: [
                    Text(
                      LibraryDuplicateStrings.duplicateImages(_locations.total),
                      style: Theme.of(context).textTheme.titleLarge,
                    ),
                    if (label != null)
                      Text(
                        label,
                        style: Theme.of(context).textTheme.bodyMedium,
                      ),
                  ],
                ),
              ),
            ),
            SizedBox(
              height: 2,
              child: _locations.loading
                  ? const LinearProgressIndicator(minHeight: 2)
                  : const Divider(height: 2),
            ),
            if (_locations.failed)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 24),
                child: Row(
                  children: [
                    const Expanded(
                      child: Text(LibraryDuplicateStrings.loadFailed),
                    ),
                    TextButton(
                      onPressed: _locations.revalidate,
                      child: const Text(LibraryDuplicateStrings.retry),
                    ),
                  ],
                ),
              ),
            Expanded(child: _buildLocations()),
          ],
        ),
      ),
    ),
  );

  Widget _buildLocations() {
    if (_locations.total == 0 && !_locations.loading && !_locations.failed) {
      return const Center(child: Text(LibraryDuplicateStrings.emptyLocations));
    }
    return AmeScrollMotion(
      child: ListView.builder(
        key: ValueKey((widget.group.digest, widget.group.byteLength)),
        itemCount: _locations.total,
        padding: const EdgeInsets.only(bottom: 16),
        itemBuilder: (context, index) {
          _requestBuiltRange(index);
          final location = _locations.itemAt(index);
          if (location == null) {
            return SizedBox(
              height: 32 + MediaQuery.textScalerOf(context).scale(56),
            );
          }
          return LibraryDuplicateLocationTile(
            key: ValueKey(location.locationId),
            location: location,
            isCurrent: location.locationId == widget.currentLocationId,
            wrapPath: true,
          );
        },
      ),
    );
  }
}

class _LocationSlice
    extends LibraryDuplicateReviewSlice<LibraryDuplicateLocation, String> {
  _LocationSlice(LibraryDuplicateLocationPage page, LibraryDuplicateGroup group)
    : pendingCount = page.pendingLocationCount ?? group.pendingLocationCount,
      super(
        items: page.locations,
        total: page.locationCount ?? group.locationCount,
        next: page.nextLocationId,
      );

  final int pendingCount;
}
