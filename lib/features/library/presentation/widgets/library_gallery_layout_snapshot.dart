import "dart:math" as math;
import "dart:typed_data";

import "../../domain/gallery_layout_manifest.dart";
import "../../domain/library_models.dart";
import "../gallery_view_options.dart";
import "../library_strings.dart";
import "justified_gallery_layout.dart";
import "library_gallery_layout.dart";
import "library_gallery_card_metrics.dart";
import "library_gallery_grouping.dart";

class LibraryGalleryLayoutSnapshot {
  LibraryGalleryLayoutSnapshot._({
    required this.manifest,
    required this.entries,
    required this.entryStartOffsets,
    required this.metrics,
    required this._itemEndOffsets,
    required this.availableWidth,
    required this.thumbnailSize,
    required this.layoutShape,
    required this.sortKey,
    required this.grouping,
    required this.minimumCellWidth,
  });

  factory LibraryGalleryLayoutSnapshot.build({
    required LibraryGalleryLayoutManifest manifest,
    required double availableWidth,
    required GalleryThumbnailSize thumbnailSize,
    GalleryLayoutShape layoutShape = GalleryLayoutShape.equalHeight,
    required LibraryGallerySortKey sortKey,
    LibraryGalleryGrouping? grouping,
    double minimumCellWidth = LibraryGalleryCardMetrics.minimumWidth,
  }) {
    final effectiveGrouping =
        grouping ?? LibraryGalleryGrouping.forSortKey(sortKey);
    String? dateKeyAt(int index) =>
        effectiveGrouping == LibraryGalleryGrouping.date
        ? _normalizedDateKey(manifest.dateKeyAt(index))
        : null;
    if (manifest.itemCount == 0 || availableWidth <= 0) {
      return LibraryGalleryLayoutSnapshot._(
        manifest: manifest,
        entries: const [],
        entryStartOffsets: Float64List(0),
        metrics: LibraryGalleryLayoutMetrics(
          contentExtent: _topPadding + _bottomPadding,
          photoRowHeight: thumbnailSize.targetExtent,
          dateAnchors: const [],
          locationOffsets: const {},
          itemOffsets: Float64List(0),
          itemIndexBase: 0,
          isQueryWide: true,
        ),
        itemEndOffsets: Float64List(0),
        availableWidth: availableWidth,
        thumbnailSize: thumbnailSize,
        layoutShape: layoutShape,
        sortKey: sortKey,
        grouping: effectiveGrouping,
        minimumCellWidth: minimumCellWidth,
      );
    }

    final entries = <LibraryGalleryLayoutSnapshotEntry>[];
    final itemOffsets = Float64List(manifest.itemCount);
    final itemEndOffsets = Float64List(manifest.itemCount);
    final dateAnchors = <LibraryGalleryDateAnchor>[];
    final targetRowHeight = thumbnailSize.targetExtent;
    var entryExtent = 0.0;
    var groupStart = 0;
    var activeDateKey = dateKeyAt(0);

    void appendEntry(LibraryGalleryLayoutSnapshotEntry entry) {
      entries.add(entry);
      entryExtent += entry.extent;
    }

    void appendGroup(int start, int end, String? dateKey) {
      final monthKey = _monthKey(dateKey);
      if (effectiveGrouping == LibraryGalleryGrouping.date) {
        final label = _dateLabel(dateKey);
        dateAnchors.add(
          LibraryGalleryDateAnchor(
            id: dateKey ?? "unknown",
            label: label,
            scrollOffset: _topPadding + entryExtent,
            year: dateKey == null
                ? null
                : int.tryParse(dateKey.substring(0, 4)),
            isUnknown: dateKey == null,
          ),
        );
        appendEntry(
          LibraryGalleryLayoutSnapshotEntry.header(
            itemIndex: start,
            monthKey: monthKey,
            dateKey: dateKey,
            label: label,
          ),
        );
      }

      if (layoutShape == GalleryLayoutShape.square) {
        final columnCount = math.max(
          1,
          ((availableWidth + LibraryGalleryLayoutEntry.spacing) /
                  (math.max(targetRowHeight, minimumCellWidth) +
                      LibraryGalleryLayoutEntry.spacing))
              .floor(),
        );
        final tileExtent =
            (availableWidth -
                LibraryGalleryLayoutEntry.spacing * (columnCount - 1)) /
            columnCount;
        for (var rowStart = start; rowStart < end; rowStart += columnCount) {
          final rowEnd = math.min(rowStart + columnCount, end);
          final rowOffset = _topPadding + entryExtent;
          final rowEndOffset =
              rowOffset + tileExtent + LibraryGalleryLayoutEntry.spacing;
          for (var itemIndex = rowStart; itemIndex < rowEnd; itemIndex++) {
            itemOffsets[itemIndex] = rowOffset;
            itemEndOffsets[itemIndex] = rowEndOffset;
          }
          appendEntry(
            LibraryGalleryLayoutSnapshotEntry.row(
              startItemIndex: rowStart,
              itemCount: rowEnd - rowStart,
              monthKey: monthKey,
              rowHeight: tileExtent,
              cellWidths: Float32List.fromList(
                List<double>.filled(rowEnd - rowStart, tileExtent),
              ),
            ),
          );
        }
        appendEntry(
          LibraryGalleryLayoutSnapshotEntry.gap(
            itemIndex: end - 1,
            monthKey: monthKey,
          ),
        );
        return;
      }

      final rows =
          JustifiedGalleryLayout(
            targetRowHeight: targetRowHeight,
            spacing: LibraryGalleryLayoutEntry.spacing,
            minimumCellWidth: minimumCellWidth,
          ).computeRows(
            itemCount: end - start,
            aspectRatioAt: (index) => manifest.aspectRatioAt(start + index),
            availableWidth: availableWidth,
          );
      for (final row in rows) {
        final rowStart = start + row.cells.first.itemIndex;
        final rowOffset = _topPadding + entryExtent;
        final rowEndOffset =
            rowOffset + row.height + LibraryGalleryLayoutEntry.spacing;
        for (var index = 0; index < row.cells.length; index++) {
          itemOffsets[rowStart + index] = rowOffset;
          itemEndOffsets[rowStart + index] = rowEndOffset;
        }
        appendEntry(
          LibraryGalleryLayoutSnapshotEntry.row(
            startItemIndex: rowStart,
            itemCount: row.cells.length,
            monthKey: monthKey,
            rowHeight: row.height,
            cellWidths: Float32List.fromList([
              for (final cell in row.cells) cell.width,
            ]),
          ),
        );
      }
      appendEntry(
        LibraryGalleryLayoutSnapshotEntry.gap(
          itemIndex: end - 1,
          monthKey: monthKey,
        ),
      );
    }

    for (var index = 1; index < manifest.itemCount; index++) {
      final dateKey = dateKeyAt(index);
      if (dateKey == activeDateKey) {
        continue;
      }
      appendGroup(groupStart, index, activeDateKey);
      groupStart = index;
      activeDateKey = dateKey;
    }
    appendGroup(groupStart, manifest.itemCount, activeDateKey);

    final entryStartOffsets = Float64List(entries.length);
    var runningEntryOffset = 0.0;
    for (var index = 0; index < entries.length; index++) {
      entryStartOffsets[index] = runningEntryOffset;
      runningEntryOffset += entries[index].extent;
    }
    var photoRowHeight = targetRowHeight;
    for (final entry in entries) {
      if (entry.isPhotoRow) {
        photoRowHeight = entry.rowHeight;
        break;
      }
    }
    final metrics = LibraryGalleryLayoutMetrics(
      contentExtent: _topPadding + runningEntryOffset + _bottomPadding,
      photoRowHeight: photoRowHeight,
      dateAnchors: List.unmodifiable(dateAnchors),
      locationOffsets: const {},
      itemOffsets: itemOffsets,
      itemIndexBase: 0,
      isQueryWide: true,
    );
    return LibraryGalleryLayoutSnapshot._(
      manifest: manifest,
      entries: List.unmodifiable(entries),
      entryStartOffsets: entryStartOffsets,
      metrics: metrics,
      itemEndOffsets: itemEndOffsets,
      availableWidth: availableWidth,
      thumbnailSize: thumbnailSize,
      layoutShape: layoutShape,
      sortKey: sortKey,
      grouping: effectiveGrouping,
      minimumCellWidth: minimumCellWidth,
    );
  }

  static const _topPadding = 18.0;
  static const _bottomPadding = 72.0;

  final LibraryGalleryLayoutManifest manifest;
  final List<LibraryGalleryLayoutSnapshotEntry> entries;
  final Float64List entryStartOffsets;
  final LibraryGalleryLayoutMetrics metrics;
  final Float64List _itemEndOffsets;
  final double availableWidth;
  final GalleryThumbnailSize thumbnailSize;
  final GalleryLayoutShape layoutShape;
  final LibraryGallerySortKey sortKey;
  final LibraryGalleryGrouping grouping;
  final double minimumCellWidth;

  bool matches({
    required LibraryGalleryLayoutManifest otherManifest,
    required double otherAvailableWidth,
    double otherMinimumCellWidth = LibraryGalleryCardMetrics.minimumWidth,
    required GalleryThumbnailSize otherThumbnailSize,
    required GalleryLayoutShape otherLayoutShape,
    required LibraryGallerySortKey otherSortKey,
    LibraryGalleryGrouping? otherGrouping,
  }) {
    return identical(manifest, otherManifest) &&
        (availableWidth - otherAvailableWidth).abs() < 0.01 &&
        (minimumCellWidth - otherMinimumCellWidth).abs() < 0.01 &&
        thumbnailSize == otherThumbnailSize &&
        layoutShape == otherLayoutShape &&
        sortKey == otherSortKey &&
        grouping ==
            (otherGrouping ?? LibraryGalleryGrouping.forSortKey(otherSortKey));
  }

  bool matchesInputs({
    required LibraryGalleryLayoutManifest otherManifest,
    required GalleryThumbnailSize otherThumbnailSize,
    required GalleryLayoutShape otherLayoutShape,
    required LibraryGallerySortKey otherSortKey,
    LibraryGalleryGrouping? otherGrouping,
    double otherMinimumCellWidth = LibraryGalleryCardMetrics.minimumWidth,
  }) {
    return identical(manifest, otherManifest) &&
        (minimumCellWidth - otherMinimumCellWidth).abs() < 0.01 &&
        thumbnailSize == otherThumbnailSize &&
        layoutShape == otherLayoutShape &&
        sortKey == otherSortKey &&
        grouping ==
            (otherGrouping ?? LibraryGalleryGrouping.forSortKey(otherSortKey));
  }

  bool canReplaceGeometry({
    required LibraryGalleryLayoutManifest otherManifest,
    required GalleryThumbnailSize otherThumbnailSize,
    required GalleryLayoutShape otherLayoutShape,
    required LibraryGallerySortKey otherSortKey,
    LibraryGalleryGrouping? otherGrouping,
  }) {
    return manifest.queryId == otherManifest.queryId &&
        manifest.revision == otherManifest.revision &&
        manifest.itemCount == otherManifest.itemCount &&
        thumbnailSize == otherThumbnailSize &&
        layoutShape == otherLayoutShape &&
        sortKey == otherSortKey &&
        grouping ==
            (otherGrouping ?? LibraryGalleryGrouping.forSortKey(otherSortKey));
  }

  int entryIndexForScrollOffset(double scrollOffset) {
    if (entryStartOffsets.isEmpty) {
      return -1;
    }
    var lower = 0;
    var upper = entryStartOffsets.length;
    while (lower < upper) {
      final middle = lower + ((upper - lower) >> 1);
      if (entryStartOffsets[middle] <= scrollOffset) {
        lower = middle + 1;
      } else {
        upper = middle;
      }
    }
    return (lower - 1).clamp(0, entries.length - 1).toInt();
  }

  ({double width, double height}) displaySizeForItemIndex(int itemIndex) {
    final fallback = (
      width: thumbnailSize.targetExtent,
      height: thumbnailSize.targetExtent,
    );
    if (itemIndex < 0 || itemIndex >= manifest.itemCount) {
      return fallback;
    }
    final entryIndex = entryIndexForScrollOffset(
      metrics.itemOffsets[itemIndex],
    );
    if (entryIndex < 0) {
      return fallback;
    }
    final entry = entries[entryIndex];
    final cellIndex = itemIndex - entry.startItemIndex;
    if (cellIndex < 0 || cellIndex >= entry.cellWidths.length) {
      return fallback;
    }
    return (width: entry.cellWidths[cellIndex], height: entry.rowHeight);
  }

  ({double leading, double content, double trailing}) loadedWindowGeometry({
    required int startItemIndex,
    required int itemCount,
  }) {
    if (manifest.itemCount == 0 || itemCount <= 0) {
      return (leading: 0, content: 0, trailing: metrics.contentExtent);
    }
    final start = startItemIndex.clamp(0, manifest.itemCount - 1).toInt();
    final end = (start + itemCount - 1)
        .clamp(start, manifest.itemCount - 1)
        .toInt();
    final leading = metrics.itemOffsets[start];
    final loadedEnd = _itemEndOffsets[end];
    return (
      leading: leading,
      content: math.max(0, loadedEnd - leading),
      trailing: math.max(0, metrics.contentExtent - loadedEnd),
    );
  }

  static String? _monthKey(String? dateKey) {
    if (dateKey == null || dateKey.length < 7) {
      return null;
    }
    return dateKey.substring(0, 7);
  }

  static String? _normalizedDateKey(String? dateKey) {
    if (dateKey == null || !_dateKeyPattern.hasMatch(dateKey)) {
      return null;
    }
    return dateKey;
  }

  static final RegExp _dateKeyPattern = RegExp(r"^\d{4}-\d{2}-\d{2}$");

  static String _dateLabel(String? dateKey) {
    if (dateKey == null) {
      return LibraryStrings.unknownCaptureDate;
    }
    final parts = dateKey.split("-");
    if (parts.length != 3) {
      return dateKey;
    }
    return "${parts[0]}年${int.tryParse(parts[1]) ?? parts[1]}月"
        "${int.tryParse(parts[2]) ?? parts[2]}日";
  }
}

class LibraryGalleryLayoutSnapshotEntry {
  const LibraryGalleryLayoutSnapshotEntry._({
    required this.extent,
    required this.startItemIndex,
    required this.itemCount,
    required this.monthKey,
    required this.rowHeight,
    required this.cellWidths,
    this.dateKey,
    this.headerLabel,
  });

  factory LibraryGalleryLayoutSnapshotEntry.header({
    required int itemIndex,
    required String? monthKey,
    required String? dateKey,
    required String label,
  }) {
    return LibraryGalleryLayoutSnapshotEntry._(
      extent: LibraryGalleryLayoutEntry.headerExtent,
      startItemIndex: itemIndex,
      itemCount: 0,
      monthKey: monthKey,
      dateKey: dateKey,
      headerLabel: label,
      rowHeight: 0,
      cellWidths: Float32List(0),
    );
  }

  factory LibraryGalleryLayoutSnapshotEntry.row({
    required int startItemIndex,
    required int itemCount,
    required String? monthKey,
    required double rowHeight,
    required Float32List cellWidths,
  }) {
    return LibraryGalleryLayoutSnapshotEntry._(
      extent: rowHeight + LibraryGalleryLayoutEntry.spacing,
      startItemIndex: startItemIndex,
      itemCount: itemCount,
      monthKey: monthKey,
      rowHeight: rowHeight,
      cellWidths: cellWidths,
    );
  }

  factory LibraryGalleryLayoutSnapshotEntry.gap({
    required int itemIndex,
    required String? monthKey,
  }) {
    return LibraryGalleryLayoutSnapshotEntry._(
      extent: LibraryGalleryLayoutEntry.groupGap,
      startItemIndex: itemIndex,
      itemCount: 0,
      monthKey: monthKey,
      rowHeight: 0,
      cellWidths: Float32List(0),
    );
  }

  final double extent;
  final int startItemIndex;
  final int itemCount;
  final String? monthKey;
  final String? dateKey;
  final String? headerLabel;
  final double rowHeight;
  final Float32List cellWidths;

  bool get isPhotoRow => itemCount > 0;
}
