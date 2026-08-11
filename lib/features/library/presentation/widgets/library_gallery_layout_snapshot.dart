import "dart:math" as math;
import "dart:typed_data";

import "../../domain/gallery_layout_manifest.dart";
import "../../domain/library_models.dart";
import "../gallery_view_options.dart";
import "../library_strings.dart";
import "library_gallery_layout.dart";

class LibraryGalleryLayoutSnapshot {
  LibraryGalleryLayoutSnapshot._({
    required this.manifest,
    required this.entries,
    required this.entryStartOffsets,
    required this.metrics,
    required this._itemEndOffsets,
    required this.availableWidth,
    required this.thumbnailSize,
    required this.sortKey,
  });

  factory LibraryGalleryLayoutSnapshot.build({
    required LibraryGalleryLayoutManifest manifest,
    required double availableWidth,
    required GalleryThumbnailSize thumbnailSize,
    required LibraryGallerySortKey sortKey,
  }) {
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
        sortKey: sortKey,
      );
    }

    final entries = <LibraryGalleryLayoutSnapshotEntry>[];
    final itemOffsets = Float64List(manifest.itemCount);
    final itemEndOffsets = Float64List(manifest.itemCount);
    final dateAnchors = <LibraryGalleryDateAnchor>[];
    final targetRowHeight = thumbnailSize.targetExtent;
    final minimumAspectRatio =
        math.min(_minimumCellWidth, availableWidth) / targetRowHeight;
    var entryExtent = 0.0;
    var groupStart = 0;
    var activeDateKey = _normalizedDateKey(manifest.dateKeyAt(0));

    void appendEntry(LibraryGalleryLayoutSnapshotEntry entry) {
      entries.add(entry);
      entryExtent += entry.extent;
    }

    void appendGroup(int start, int end, String? dateKey) {
      final monthKey = _monthKey(dateKey);
      if (sortKey != LibraryGallerySortKey.fileName) {
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

      var rowStart = start;
      final rowRatios = <double>[];
      var naturalRowWidth = 0.0;

      void appendRow({required bool shouldFillWidth}) {
        if (rowRatios.isEmpty) {
          return;
        }
        final availableImageWidth =
            availableWidth -
            LibraryGalleryLayoutEntry.spacing * (rowRatios.length - 1);
        final naturalImageWidth = rowRatios.fold<double>(
          0,
          (sum, ratio) => sum + targetRowHeight * ratio,
        );
        final widthScale = shouldFillWidth && naturalImageWidth > 0
            ? availableImageWidth / naturalImageWidth
            : 1.0;
        final widths = Float32List(rowRatios.length);
        var occupiedWidth = 0.0;
        for (var index = 0; index < rowRatios.length; index++) {
          final isLast = index == rowRatios.length - 1;
          final naturalWidth = targetRowHeight * rowRatios[index];
          final width = shouldFillWidth && isLast
              ? availableImageWidth - occupiedWidth
              : (naturalWidth * widthScale).clamp(0.0, availableImageWidth);
          widths[index] = width;
          occupiedWidth += width;
        }
        final rowOffset = _topPadding + entryExtent;
        final rowEndOffset =
            rowOffset + targetRowHeight + LibraryGalleryLayoutEntry.spacing;
        for (var index = 0; index < rowRatios.length; index++) {
          itemOffsets[rowStart + index] = rowOffset;
          itemEndOffsets[rowStart + index] = rowEndOffset;
        }
        appendEntry(
          LibraryGalleryLayoutSnapshotEntry.row(
            startItemIndex: rowStart,
            itemCount: rowRatios.length,
            monthKey: monthKey,
            rowHeight: targetRowHeight,
            cellWidths: widths,
          ),
        );
        rowStart += rowRatios.length;
        rowRatios.clear();
        naturalRowWidth = 0;
      }

      for (var itemIndex = start; itemIndex < end; itemIndex++) {
        final ratio = math.max(
          _normalizeAspectRatio(manifest.aspectRatioAt(itemIndex)),
          minimumAspectRatio,
        );
        final naturalWidth = targetRowHeight * ratio;
        final nextWidth = naturalRowWidth == 0
            ? naturalWidth
            : naturalRowWidth +
                  LibraryGalleryLayoutEntry.spacing +
                  naturalWidth;
        if (naturalRowWidth > 0 && nextWidth > availableWidth) {
          appendRow(shouldFillWidth: true);
        }
        rowRatios.add(ratio);
        naturalRowWidth = naturalRowWidth == 0
            ? naturalWidth
            : naturalRowWidth +
                  LibraryGalleryLayoutEntry.spacing +
                  naturalWidth;
      }
      appendRow(shouldFillWidth: false);
      appendEntry(
        LibraryGalleryLayoutSnapshotEntry.gap(
          itemIndex: end - 1,
          monthKey: monthKey,
        ),
      );
    }

    for (var index = 1; index < manifest.itemCount; index++) {
      final dateKey = _normalizedDateKey(manifest.dateKeyAt(index));
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
    final metrics = LibraryGalleryLayoutMetrics(
      contentExtent: _topPadding + runningEntryOffset + _bottomPadding,
      photoRowHeight: targetRowHeight,
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
      sortKey: sortKey,
    );
  }

  static const _topPadding = 18.0;
  static const _bottomPadding = 72.0;
  static const _minimumCellWidth = 48.0;

  final LibraryGalleryLayoutManifest manifest;
  final List<LibraryGalleryLayoutSnapshotEntry> entries;
  final Float64List entryStartOffsets;
  final LibraryGalleryLayoutMetrics metrics;
  final Float64List _itemEndOffsets;
  final double availableWidth;
  final GalleryThumbnailSize thumbnailSize;
  final LibraryGallerySortKey sortKey;

  bool matches({
    required LibraryGalleryLayoutManifest otherManifest,
    required double otherAvailableWidth,
    required GalleryThumbnailSize otherThumbnailSize,
    required LibraryGallerySortKey otherSortKey,
  }) {
    return identical(manifest, otherManifest) &&
        (availableWidth - otherAvailableWidth).abs() < 0.01 &&
        thumbnailSize == otherThumbnailSize &&
        sortKey == otherSortKey;
  }

  bool matchesInputs({
    required LibraryGalleryLayoutManifest otherManifest,
    required GalleryThumbnailSize otherThumbnailSize,
    required LibraryGallerySortKey otherSortKey,
  }) {
    return identical(manifest, otherManifest) &&
        thumbnailSize == otherThumbnailSize &&
        sortKey == otherSortKey;
  }

  bool canReplaceGeometry({
    required LibraryGalleryLayoutManifest otherManifest,
    required GalleryThumbnailSize otherThumbnailSize,
    required LibraryGallerySortKey otherSortKey,
  }) {
    return manifest.queryId == otherManifest.queryId &&
        manifest.revision == otherManifest.revision &&
        manifest.itemCount == otherManifest.itemCount &&
        thumbnailSize == otherThumbnailSize &&
        sortKey == otherSortKey;
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

  static double _normalizeAspectRatio(double value) {
    if (!value.isFinite || value <= 0) {
      return 1;
    }
    return value.clamp(0.2, 5.0).toDouble();
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
