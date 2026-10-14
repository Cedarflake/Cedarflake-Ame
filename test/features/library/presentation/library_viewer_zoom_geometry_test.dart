import "dart:ui";

import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_zoom_geometry.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("window fit is 100 percent for small, tall and wide images", () {
    for (final imageSize in [
      const Size(60, 60),
      const Size(100, 200),
      const Size(4000, 2000),
      const Size(8000, 1000),
    ]) {
      final geometry = LibraryViewerZoomGeometry(
        imageSize: imageSize,
        viewportSize: const Size(1000, 600),
      );
      expect(geometry.percentageForScale(1), 100);
      expect(geometry.percentageForScale(1.25), 125);
      expect(geometry.percentageForScale(0.5), 50);
      expect(geometry.isActualSize(geometry.actualSizeScale), isTrue);
    }
  });

  test("actual pixels stay distinct from the window-relative percentage", () {
    const small = LibraryViewerZoomGeometry(
      imageSize: Size(60, 60),
      viewportSize: Size(600, 600),
    );
    const large = LibraryViewerZoomGeometry(
      imageSize: Size(4000, 2000),
      viewportSize: Size(1000, 600),
    );
    expect(small.fitImageScale, 10);
    expect(small.actualSizeScale, 0.1);
    expect(small.percentageForScale(small.actualSizeScale), 10);
    expect(large.fitImageScale, 0.25);
    expect(large.actualSizeScale, 4);
    expect(large.percentageForScale(large.actualSizeScale), 400);
    expect(small.isActualSize(1), isFalse);
    expect(large.isActualSize(1), isFalse);
    for (final geometry in [small, large]) {
      final actualValue = geometry.sliderValueForScale(
        geometry.actualSizeScale,
      );
      expect(
        geometry.scaleForSliderValue(actualValue),
        closeTo(geometry.actualSizeScale, 0.000001),
      );
    }
  });

  test("portrait fit follows viewport height and resize keeps fit at 100", () {
    const geometry = LibraryViewerZoomGeometry(
      imageSize: Size(100, 200),
      viewportSize: Size(800, 600),
    );
    const resized = LibraryViewerZoomGeometry(
      imageSize: Size(100, 200),
      viewportSize: Size(800, 1000),
    );
    expect(geometry.fitImageScale, 3);
    expect(resized.fitImageScale, 5);
    expect(geometry.percentageForScale(1), 100);
    expect(resized.percentageForScale(1), 100);
  });
}
