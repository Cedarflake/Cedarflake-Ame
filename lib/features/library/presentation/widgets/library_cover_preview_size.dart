import "dart:math" as math;

typedef LibraryCoverPreviewSize = ({int previewEdge, int decodeWidth});

LibraryCoverPreviewSize libraryCoverPreviewSize({
  required int imageWidth,
  required int imageHeight,
  required double width,
  required double height,
  required double devicePixelRatio,
}) {
  var renderedWidth = width;
  var renderedHeight = height;
  if (imageWidth > 0 && imageHeight > 0) {
    final scale = math.max(width / imageWidth, height / imageHeight);
    renderedWidth = imageWidth * scale;
    renderedHeight = imageHeight * scale;
  }
  return (
    previewEdge: _bucket(
      math.max(renderedWidth, renderedHeight) * devicePixelRatio,
    ),
    decodeWidth: _bucket(renderedWidth * devicePixelRatio),
  );
}

int _bucket(double extent) {
  for (final bucket in const [128, 256, 512, 1024]) {
    if (extent <= bucket) return bucket;
  }
  return 1024;
}
