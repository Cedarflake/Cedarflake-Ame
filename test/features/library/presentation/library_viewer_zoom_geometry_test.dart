import "dart:ui";

import "package:cedarflake_ame/features/library/presentation/widgets/library_viewer_zoom_geometry.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "actual pixels are 100 percent for tiny, small, tall and wide images",
    () {
      for (final imageSize in [
        const Size(1, 1),
        const Size(60, 60),
        const Size(100, 200),
        const Size(4000, 2000),
        const Size(8000, 1000),
      ]) {
        final geometry = LibraryViewerZoomGeometry(
          imageSize: imageSize,
          viewportSize: const Size(1000, 600),
        );
        final actualScale = geometry.actualSizeScale;
        expect(
          geometry.percentageForScale(actualScale),
          closeTo(100, 0.000001),
        );
        expect(
          geometry.percentageForScale(actualScale * 1.25),
          closeTo(125, 0.000001),
        );
        expect(
          geometry.percentageForScale(actualScale * 0.5),
          closeTo(50, 0.000001),
        );
        expect(actualScale, greaterThanOrEqualTo(geometry.minimumScale));
        expect(geometry.isActualSize(geometry.actualSizeScale), isTrue);
      }
    },
  );

  test("fit percentages retain their source-pixel reference", () {
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
    expect(small.percentageForScale(small.actualSizeScale), 100);
    expect(small.percentageForScale(1), 1000);
    expect(large.fitImageScale, 0.25);
    expect(large.actualSizeScale, 4);
    expect(large.percentageForScale(large.actualSizeScale), 100);
    expect(large.percentageForScale(1), 25);
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

  test(
    "explicit portrait fit follows viewport height without relabeling zoom",
    () {
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
      expect(geometry.percentageForScale(1), 300);
      expect(resized.percentageForScale(1), 500);
    },
  );
}
