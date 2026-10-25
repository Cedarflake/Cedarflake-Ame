import "package:flutter/material.dart";
import "package:flutter/semantics.dart";

import "library_gallery_loading_region.dart";
import "library_sidebar_content.dart";

class LibraryGalleryViewport extends StatelessWidget {
  const LibraryGalleryViewport({
    required this.header,
    required this.body,
    required this.isEmpty,
    required this.isLoading,
    this.failure,
    this.footer,
    this.notice,
    this.loadingDelay = Duration.zero,
    super.key,
  });

  final Widget header;
  final Widget body;
  final Widget? failure;
  final Widget? footer;
  final Widget? notice;
  final bool isEmpty;
  final bool isLoading;
  final Duration loadingDelay;

  @override
  Widget build(BuildContext context) {
    return Stack(
      fit: StackFit.expand,
      children: [
        if (isEmpty)
          Positioned.fill(
            child: Semantics(
              sortKey: const OrdinalSortKey(1),
              child: LibrarySidebarContent(
                alignment: Alignment.topCenter,
                child: body,
              ),
            ),
          ),
        Column(
          children: [
            Semantics(sortKey: const OrdinalSortKey(0), child: header),
            ?failure,
            Expanded(
              child: LibraryGalleryLoadingRegion(
                isLoading: isLoading,
                delay: loadingDelay,
                child: Stack(
                  fit: StackFit.expand,
                  children: [if (!isEmpty) body, ?notice],
                ),
              ),
            ),
          ],
        ),
        if (footer case final content?)
          Positioned(
            left: 0,
            right: 0,
            bottom: 0,
            child: LibrarySidebarContent(
              alignment: Alignment.topCenter,
              child: content,
            ),
          ),
      ],
    );
  }
}
