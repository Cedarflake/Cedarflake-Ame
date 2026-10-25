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
    super.key,
  });

  final Widget header;
  final Widget body;
  final Widget? failure;
  final Widget? footer;
  final bool isEmpty;
  final bool isLoading;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Expanded(
          child: Stack(
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
                      child: isEmpty ? const SizedBox.shrink() : body,
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
        if (footer case final content?)
          LibrarySidebarContent(alignment: Alignment.topCenter, child: content),
      ],
    );
  }
}
