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
        if (localTime == null || localTime.length < 10) {
          return null;
        }
        final value = localTime.substring(0, 10);
        final match = RegExp(r"^\d{4}-\d{2}-\d{2}$").firstMatch(value);
        return match == null ? null : value;
      case LibraryGallerySortKey.createdTime:
        final createdUnixMs = asset.createdUnixMs;
        return createdUnixMs == null ? null : _unixDateKey(createdUnixMs);
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
