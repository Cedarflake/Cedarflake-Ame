import "dart:typed_data";

import "package:cedarflake_ame/features/library/domain/gallery_buffer_equality.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("compares every byte within aligned and unaligned column views", () {
    for (final (leftOffset, rightOffset) in [(0, 0), (8, 16), (0, 3), (1, 3)]) {
      for (final length in [0, 1, 7, 8, 9, 17]) {
        final source = Uint8List.fromList(
          List.generate(length + leftOffset, (index) => 255 - index),
        );
        final left = Uint8List.sublistView(source, leftOffset);
        final copy = Uint8List(length + rightOffset)
          ..setRange(rightOffset, length + rightOffset, left);
        final right = Uint8List.sublistView(copy, rightOffset);
        expect(galleryBuffersEqual(left, right), isTrue);
        expect(galleryBuffersEqual(left, Uint8List(length + 1)), isFalse);
        for (var index = 0; index < length; index++) {
          right[index] ^= 128;
          expect(galleryBuffersEqual(left, right), isFalse);
          right[index] ^= 128;
        }
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
