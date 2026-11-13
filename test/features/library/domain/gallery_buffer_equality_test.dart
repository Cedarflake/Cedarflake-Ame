import "dart:typed_data";

import "package:cedarflake_ame/features/library/domain/gallery_buffer_equality.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("compares every byte within aligned and unaligned column views", () {
    for (final length in [0, 1, 7, 8, 9, 17]) {
      final source = Uint8List.fromList(
        List.generate(length + 4, (index) => 255 - index),
      );
      final left = Uint8List.sublistView(source, 1, length + 1);
      final copy = Uint8List(length + 6)..setRange(3, length + 3, left);
      final right = Uint8List.sublistView(copy, 3, length + 3);
      expect(galleryBuffersEqual(left, right), isTrue);
      expect(galleryBuffersEqual(left, Uint8List(length + 1)), isFalse);
      for (var index = 0; index < length; index++) {
        right[index] ^= 128;
        expect(galleryBuffersEqual(left, right), isFalse);
        right[index] ^= 128;
      }
    }
  });

  test("retains exact integer-column equality beyond a complete word", () {
    final dimensions = Uint16List.fromList([1000, 2000, 3000, 4000, 65535]);
    final dimensionsCopy = Uint16List.fromList(dimensions);
    final offsets = Uint32List.fromList([0, 36, 72]);
    final offsetsCopy = Uint32List.fromList(offsets);
    expect(galleryBuffersEqual(dimensions, dimensionsCopy), isTrue);
    expect(galleryBuffersEqual(offsets, offsetsCopy), isTrue);
    dimensionsCopy.last = 32767;
    offsetsCopy.last = 73;
    expect(galleryBuffersEqual(dimensions, dimensionsCopy), isFalse);
    expect(galleryBuffersEqual(offsets, offsetsCopy), isFalse);
  });
}
