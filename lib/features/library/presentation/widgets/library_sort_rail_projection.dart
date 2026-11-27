import "../../../../app/presentation/ame_localizations.dart";
import "../../domain/library_models.dart";
import "library_gallery_layout.dart";
import "timeline_rail_model.dart";

class LibrarySortRailProjection {
  const LibrarySortRailProjection({required this.timeline, this.metrics});

  final LibraryTimeline? timeline;
  final LibraryGalleryLayoutMetrics? metrics;

  List<TimelineRailBucket> buckets(
    AppLocalizations strings,
    double maximumOffset,
  ) {
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

    double sectionStart(LibraryScrollSection section) {
      if (section.kind == LibraryScrollSectionKind.name &&
          section.startOrdinal == 0) {
        return 0;
      }
      return offset(section.startOrdinal);
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
            LibraryScrollSectionKind.newArrivals => strings.recentlyAddedCount(
              section.itemCount,
            ),
            LibraryScrollSectionKind.earlier => strings.previouslyAddedCount(
              section.itemCount,
            ),
          },
          axisLabel: switch (section.kind) {
            LibraryScrollSectionKind.name => _name(section.key),
            LibraryScrollSectionKind.newArrivals => strings.recentlyAdded,
            LibraryScrollSectionKind.earlier =>
              hasArrivals ? strings.previouslyAdded : null,
          },
          highlighted: section.kind == LibraryScrollSectionKind.newArrivals,
          year: null,
          scrollOffset: sectionStart(section),
          contentExtent: (sectionEnd(section) - sectionStart(section)).clamp(
            0.0,
            double.infinity,
          ),
        ),
    ];
  }

  String _name(String key) => switch (key) {
    "0-9" => "0–9",
    "symbols-before" || "symbols-after" => "#",
    _ => key,
  };
}
