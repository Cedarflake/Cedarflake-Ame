import "package:flutter/material.dart";

import "annotated_time_rail.dart";

class LibraryGalleryScrollSurface extends StatelessWidget {
  const LibraryGalleryScrollSurface({
    required this.controller,
    required this.isOrganizing,
    required this.gallery,
    this.timeNavigation,
    super.key,
  });

  final ScrollController controller;
  final bool isOrganizing;
  final Widget gallery;
  final Widget? timeNavigation;

  @override
  Widget build(BuildContext context) {
    if (isOrganizing) {
      return ScrollbarTheme(
        data: ScrollbarTheme.of(context).copyWith(
          crossAxisMargin: 8,
          mainAxisMargin: 8,
          thickness: const WidgetStatePropertyAll(8),
        ),
        child: Scrollbar(
          key: const Key("library-gallery-scrollbar"),
          controller: controller,
          scrollbarOrientation: ScrollbarOrientation.right,
          thumbVisibility: true,
          interactive: true,
          child: gallery,
        ),
      );
    }
    return Row(
      children: [
        Expanded(child: gallery),
        SizedBox(width: AnnotatedTimeRail.width, child: timeNavigation),
      ],
    );
  }
}
