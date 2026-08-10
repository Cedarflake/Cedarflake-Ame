import "../../domain/library_models.dart";
import "../gallery_view_options.dart";
import "../library_strings.dart";
import "justified_gallery_layout.dart";

class LibraryGalleryLayoutEntry {
  const LibraryGalleryLayoutEntry({
    required this.extent,
    required this.monthKey,
    this.dateKey,
    this.headerLabel,
    this.rowHeight = 0,
    this.cells = const [],
    this.firstLocationId,
  });

  static const spacing = 6.0;
  static const headerExtent = 40.0;
  static const groupGap = 18.0;

  final double extent;
  final String? monthKey;
  final String? dateKey;
  final String? headerLabel;
  final double rowHeight;
  final List<LibraryGalleryLayoutCell> cells;
  final String? firstLocationId;

  static List<LibraryGalleryLayoutEntry> build({
    required List<LibraryAsset> assets,
    required double availableWidth,
    required GalleryLayoutShape layoutShape,
    required GalleryThumbnailSize thumbnailSize,
    required LibraryGallerySortKey sortKey,
  }) {
    if (assets.isEmpty || availableWidth <= 0) {
      return const [];
    }
    final groups = <_LibraryGalleryDateGroup>[];
    String? activeDateKey;
    var activeAssets = <LibraryAsset>[];
    var hasGroup = false;
    for (final asset in assets) {
      final dateKey = _dateKey(asset, sortKey);
      if (hasGroup && dateKey != activeDateKey) {
        groups.add(_LibraryGalleryDateGroup(activeDateKey, activeAssets));
        activeAssets = [];
      }
      activeDateKey = dateKey;
      activeAssets.add(asset);
      hasGroup = true;
    }
    if (hasGroup) {
      groups.add(_LibraryGalleryDateGroup(activeDateKey, activeAssets));
    }

    final entries = <LibraryGalleryLayoutEntry>[];
    for (final group in groups) {
      final monthKey = group.dateKey?.substring(0, 7);
      if (sortKey != LibraryGallerySortKey.fileName) {
        entries.add(
          LibraryGalleryLayoutEntry(
            extent: headerExtent,
            monthKey: monthKey,
            dateKey: group.dateKey,
            headerLabel: _dateLabel(group.dateKey),
            firstLocationId: group.assets.first.locationId,
          ),
        );
      }
      if (layoutShape == GalleryLayoutShape.equalHeight) {
        final rows =
            JustifiedGalleryLayout(
              targetRowHeight: thumbnailSize.targetExtent,
              spacing: spacing,
            ).compute(
              aspectRatios: [
                for (final asset in group.assets) _aspectRatio(asset),
              ],
              availableWidth: availableWidth,
            );
        for (final row in rows) {
          entries.add(
            LibraryGalleryLayoutEntry(
              extent: row.height + spacing,
              monthKey: monthKey,
              rowHeight: row.height,
              firstLocationId:
                  group.assets[row.cells.first.itemIndex].locationId,
              cells: [
                for (final cell in row.cells)
                  LibraryGalleryLayoutCell(
                    asset: group.assets[cell.itemIndex],
                    width: cell.width,
                  ),
              ],
            ),
          );
        }
      } else {
        final columnCount =
            ((availableWidth + spacing) /
                    (thumbnailSize.targetExtent + spacing))
                .floor()
                .clamp(1, group.assets.length)
                .toInt();
        final tileSize =
            (availableWidth - spacing * (columnCount - 1)) / columnCount;
        for (var start = 0; start < group.assets.length; start += columnCount) {
          final end = (start + columnCount).clamp(0, group.assets.length);
          entries.add(
            LibraryGalleryLayoutEntry(
              extent: tileSize + spacing,
              monthKey: monthKey,
              rowHeight: tileSize,
              firstLocationId: group.assets[start].locationId,
              cells: [
                for (final asset in group.assets.sublist(start, end))
                  LibraryGalleryLayoutCell(asset: asset, width: tileSize),
              ],
            ),
          );
        }
      }
      entries.add(
        LibraryGalleryLayoutEntry(
          extent: groupGap,
          monthKey: monthKey,
          firstLocationId: group.assets.last.locationId,
        ),
      );
    }
    return entries;
  }

  static double _aspectRatio(LibraryAsset asset) {
    if (asset.width <= 0 || asset.height <= 0) {
      return 1;
    }
    return asset.width / asset.height;
  }

  static String? _dateKey(LibraryAsset asset, LibraryGallerySortKey sortKey) {
    switch (sortKey) {
      case LibraryGallerySortKey.captureTime:
        final localTime = asset.captureTime?.localTime;
        if (localTime != null && localTime.length >= 10) {
          final value = localTime.substring(0, 10);
          final match = RegExp(r"^\d{4}-\d{2}-\d{2}$").firstMatch(value);
          if (match != null) {
            return value;
          }
        }
        final createdUnixMs = asset.createdUnixMs;
        return _unixDateKey(createdUnixMs ?? asset.modifiedUnixMs);
      case LibraryGallerySortKey.createdTime:
        return _unixDateKey(asset.createdUnixMs ?? asset.modifiedUnixMs);
      case LibraryGallerySortKey.modifiedTime:
        return _unixDateKey(asset.modifiedUnixMs);
      case LibraryGallerySortKey.fileName:
        return null;
    }
  }

  static String _unixDateKey(int unixMs) {
    final date = DateTime.fromMillisecondsSinceEpoch(unixMs);
    return "${date.year.toString().padLeft(4, '0')}-"
        "${date.month.toString().padLeft(2, '0')}-"
        "${date.day.toString().padLeft(2, '0')}";
  }

  static String _dateLabel(String? dateKey) {
    if (dateKey == null) {
      return LibraryStrings.unknownCaptureDate;
    }
    final parts = dateKey.split("-").map(int.parse).toList(growable: false);
    return "${parts[0]}年${parts[1]}月${parts[2]}日";
  }
}

class LibraryGalleryLayoutMetrics {
  LibraryGalleryLayoutMetrics({
    required this.contentExtent,
    required this.photoRowHeight,
    required this.dateAnchors,
    required this.locationOffsets,
    required this.itemOffsets,
    this.itemIndexBase = 0,
    this.isQueryWide = false,
  });

  factory LibraryGalleryLayoutMetrics.fromEntries(
    List<LibraryGalleryLayoutEntry> entries, {
    required double topPadding,
    required double bottomPadding,
    int itemIndexBase = 0,
    bool isQueryWide = false,
  }) {
    final anchors = <LibraryGalleryDateAnchor>[];
    final offsets = <String, double>{};
    final itemOffsets = <double>[];
    var runningOffset = topPadding;
    var photoRowHeight = 0.0;
    for (final entry in entries) {
      final dateKey = entry.dateKey;
      final headerLabel = entry.headerLabel;
      if (dateKey != null || headerLabel != null) {
        anchors.add(
          LibraryGalleryDateAnchor(
            id: dateKey ?? "unknown",
            label: headerLabel ?? LibraryStrings.unknownCaptureDate,
            scrollOffset: runningOffset,
            year: dateKey == null
                ? null
                : int.tryParse(dateKey.substring(0, 4)),
            isUnknown: dateKey == null,
          ),
        );
      }
      if (entry.cells.isNotEmpty) {
        photoRowHeight = entry.rowHeight;
        for (final cell in entry.cells) {
          offsets[cell.asset.locationId] = runningOffset;
          itemOffsets.add(runningOffset);
        }
      }
      runningOffset += entry.extent;
    }
    return LibraryGalleryLayoutMetrics(
      contentExtent: runningOffset + bottomPadding,
      photoRowHeight: photoRowHeight,
      dateAnchors: List.unmodifiable(anchors),
      locationOffsets: Map.unmodifiable(offsets),
      itemOffsets: List.unmodifiable(itemOffsets),
      itemIndexBase: itemIndexBase,
      isQueryWide: isQueryWide,
    );
  }

  final double contentExtent;
  final double photoRowHeight;
  final List<LibraryGalleryDateAnchor> dateAnchors;
  final Map<String, double> locationOffsets;
  final List<double> itemOffsets;
  final int itemIndexBase;
  final bool isQueryWide;

  int get itemCount => itemOffsets.length;

  double? offsetForLocation(String? locationId) {
    if (locationId == null) {
      return null;
    }
    return locationOffsets[locationId];
  }

  double? offsetForItemIndex(int itemIndex) {
    if (itemIndex < 0 || itemIndex >= itemOffsets.length) {
      return null;
    }
    return itemOffsets[itemIndex];
  }

  double? offsetForGlobalItemIndex(int itemIndex) {
    return offsetForItemIndex(itemIndex - itemIndexBase);
  }

  bool containsGlobalItemIndex(double itemIndex) {
    return itemIndex >= itemIndexBase &&
        itemIndex < itemIndexBase + itemOffsets.length;
  }

  int? rowStartGlobalItemIndex(int itemIndex) {
    final localItemIndex = itemIndex - itemIndexBase;
    if (localItemIndex < 0 || localItemIndex >= itemOffsets.length) {
      return null;
    }
    return itemIndexBase +
        itemIndexForScrollOffset(itemOffsets[localItemIndex]);
  }

  int? rowEndGlobalItemIndexExclusive(int itemIndex) {
    final localItemIndex = itemIndex - itemIndexBase;
    if (localItemIndex < 0 || localItemIndex >= itemOffsets.length) {
      return null;
    }
    final rowOffset = itemOffsets[localItemIndex];
    var endIndex = localItemIndex + 1;
    while (endIndex < itemOffsets.length &&
        (itemOffsets[endIndex] - rowOffset).abs() < 0.01) {
      endIndex += 1;
    }
    return itemIndexBase + endIndex;
  }

  int itemIndexForScrollOffset(double scrollOffset) {
    if (itemOffsets.isEmpty) {
      return 0;
    }
    var lower = 0;
    var upper = itemOffsets.length;
    while (lower < upper) {
      final middle = lower + ((upper - lower) >> 1);
      if (itemOffsets[middle] <= scrollOffset) {
        lower = middle + 1;
      } else {
        upper = middle;
      }
    }
    var index = (lower - 1).clamp(0, itemOffsets.length - 1).toInt();
    final rowOffset = itemOffsets[index];
    while (index > 0 && (itemOffsets[index - 1] - rowOffset).abs() < 0.01) {
      index -= 1;
    }
    return index;
  }

  bool hasSameGeometry(LibraryGalleryLayoutMetrics other) {
    if (identical(this, other)) {
      return true;
    }
    if ((contentExtent - other.contentExtent).abs() > 0.01 ||
        (photoRowHeight - other.photoRowHeight).abs() > 0.01 ||
        dateAnchors.length != other.dateAnchors.length ||
        locationOffsets.length != other.locationOffsets.length ||
        itemOffsets.length != other.itemOffsets.length ||
        itemIndexBase != other.itemIndexBase ||
        isQueryWide != other.isQueryWide) {
      return false;
    }
    for (var index = 0; index < dateAnchors.length; index++) {
      final anchor = dateAnchors[index];
      final otherAnchor = other.dateAnchors[index];
      if (anchor.id != otherAnchor.id ||
          (anchor.scrollOffset - otherAnchor.scrollOffset).abs() > 0.01) {
        return false;
      }
    }
    for (final entry in locationOffsets.entries) {
      final otherOffset = other.locationOffsets[entry.key];
      if (otherOffset == null || (entry.value - otherOffset).abs() > 0.01) {
        return false;
      }
    }
    for (var index = 0; index < itemOffsets.length; index++) {
      if ((itemOffsets[index] - other.itemOffsets[index]).abs() > 0.01) {
        return false;
      }
    }
    return true;
  }
}

class LibraryGalleryDateAnchor {
  const LibraryGalleryDateAnchor({
    required this.id,
    required this.label,
    required this.scrollOffset,
    required this.year,
    required this.isUnknown,
  });

  final String id;
  final String label;
  final double scrollOffset;
  final int? year;
  final bool isUnknown;
}

class LibraryGalleryLayoutCell {
  const LibraryGalleryLayoutCell({required this.asset, required this.width});

  final LibraryAsset asset;
  final double width;
}

class _LibraryGalleryDateGroup {
  const _LibraryGalleryDateGroup(this.dateKey, this.assets);

  final String? dateKey;
  final List<LibraryAsset> assets;
}
