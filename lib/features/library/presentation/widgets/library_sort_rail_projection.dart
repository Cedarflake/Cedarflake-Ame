import "../../domain/library_models.dart";
import "library_gallery_layout.dart";
import "timeline_rail_model.dart";

class LibrarySortRailProjection {
  const LibrarySortRailProjection({required this.timeline, this.metrics});

  final LibraryTimeline? timeline;
  final LibraryGalleryLayoutMetrics? metrics;

  List<TimelineRailBucket> buckets(double maximumOffset) {
    final summary = timeline;
    if (summary == null || summary.totalItems == 0) return const [];
    final sections = summary.scrollSections;
    final hasArrivals = sections.any(
      (section) => section.kind == LibraryScrollSectionKind.newArrivals,
    );
    if (!hasArrivals &&
        sections.every(
          (section) => section.kind == LibraryScrollSectionKind.earlier,
        )) {
      return const [];
    }
    double offset(int ordinal) {
      final layout = metrics;
      if (layout != null && layout.isQueryWide) {
        return layout.offsetForGlobalItemIndex(ordinal) ?? layout.contentExtent;
      }
      return ordinal / summary.totalItems * maximumOffset;
    }

    double sectionEnd(LibraryScrollSection section) {
      final endOrdinal = section.startOrdinal + section.itemCount;
      final layout = metrics;
      if (section.kind == LibraryScrollSectionKind.newArrivals &&
          section.itemCount > 0 &&
          layout != null &&
          layout.isQueryWide) {
        final rowEnd = layout.rowEndGlobalItemIndexExclusive(endOrdinal - 1);
        if (rowEnd != null) return offset(rowEnd);
      }
      return offset(endOrdinal);
    }

    return [
      for (final section in sections)
        TimelineRailBucket(
          id: "sort-${section.key}",
          label: switch (section.kind) {
            LibraryScrollSectionKind.name => _name(section.key),
            LibraryScrollSectionKind.newArrivals =>
              "新加入 · ${section.itemCount} 张",
            LibraryScrollSectionKind.earlier => "此前加入 · ${section.itemCount} 张",
          },
          axisLabel: switch (section.kind) {
            LibraryScrollSectionKind.name => _name(section.key),
            LibraryScrollSectionKind.newArrivals => "新加入",
            LibraryScrollSectionKind.earlier => hasArrivals ? "此前" : null,
          },
          highlighted: section.kind == LibraryScrollSectionKind.newArrivals,
          year: null,
          scrollOffset: offset(section.startOrdinal),
          contentExtent: (sectionEnd(section) - offset(section.startOrdinal))
              .clamp(0.0, double.infinity),
        ),
    ];
  }

  String _name(String key) => switch (key) {
    "0-9" => "0–9",
    "symbols-before" || "symbols-after" => "#",
    _ => key,
  };
}
