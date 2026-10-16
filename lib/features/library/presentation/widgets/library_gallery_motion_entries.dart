import "package:flutter/material.dart";

import "library_gallery_layout.dart";
import "library_gallery_layout_motion.dart";

class LibraryGalleryMotionRow extends StatelessWidget {
  const LibraryGalleryMotionRow({
    required this.extent,
    required this.width,
    required this.height,
    required this.offset,
    required this.cellWidths,
    required this.identityAt,
    required this.builder,
    super.key,
  });

  final double extent;
  final double width;
  final double height;
  final double offset;
  final List<double> cellWidths;
  final Object Function(int index) identityAt;
  final Widget Function(int index, Size size) builder;

  @override
  Widget build(BuildContext context) {
    var left = 0.0;
    final children = <Widget>[];
    for (var index = 0; index < cellWidths.length; index++) {
      if (index > 0) {
        children.add(const SizedBox(width: LibraryGalleryLayoutEntry.spacing));
        left += LibraryGalleryLayoutEntry.spacing;
      }
      final identity = identityAt(index);
      children.add(
        LibraryGalleryMotionItem(
          key: ValueKey(identity),
          identity: identity,
          rect: Rect.fromLTWH(left, offset, cellWidths[index], height),
          builder: (context, size) => builder(index, size),
        ),
      );
      left += cellWidths[index];
    }
    return SizedBox(
      height: extent,
      child: Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          height: height,
          child: OverflowBox(
            alignment: Alignment.topLeft,
            minWidth: width + 0.5,
            maxWidth: width + 0.5,
            minHeight: height,
            maxHeight: height,
            child: Row(children: children),
          ),
        ),
      ),
    );
  }
}

class LibraryGalleryMotionHeader extends StatelessWidget {
  const LibraryGalleryMotionHeader({
    required this.label,
    required this.dateKey,
    required this.rect,
    super.key,
  });

  final String label;
  final String? dateKey;
  final Rect rect;

  @override
  Widget build(BuildContext context) => LibraryGalleryMotionItem(
    identity: (header: dateKey),
    rect: rect,
    builder: (context, size) => Align(
      alignment: Alignment.centerLeft,
      child: Semantics(
        header: true,
        child: Text(
          label,
          key: ValueKey("gallery-date-${dateKey ?? 'unknown'}"),
          style: Theme.of(context).textTheme.titleSmall,
        ),
      ),
    ),
  );
}
