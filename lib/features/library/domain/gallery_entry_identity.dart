import "dart:convert";
import "dart:typed_data";

/// Compact card identities are independent of the physical cover locations.
class GalleryEntryIdentityBlock {
  GalleryEntryIdentityBlock(List<String> identities)
    : offsets = Uint32List(identities.length + 1) {
    final buffer = BytesBuilder(copy: false);
    var length = 0;
    for (var index = 0; index < identities.length; index++) {
      final encoded = utf8.encode(identities[index]);
      if (encoded.isEmpty || encoded.length > 128) {
        throw ArgumentError.value(identities[index], "identity");
      }
      buffer.add(encoded);
      length += encoded.length;
      offsets[index + 1] = length;
    }
    bytes = buffer.takeBytes();
  }

  late final Uint8List bytes;
  final Uint32List offsets;

  int get length => offsets.length - 1;
  int get byteLength => bytes.lengthInBytes + offsets.lengthInBytes;

  String at(int index) => utf8.decode(
    Uint8List.sublistView(bytes, offsets[index], offsets[index + 1]),
  );

  bool sameAs(GalleryEntryIdentityBlock other) {
    if (bytes.length != other.bytes.length ||
        offsets.length != other.offsets.length) {
      return false;
    }
    for (var index = 0; index < bytes.length; index++) {
      if (bytes[index] != other.bytes[index]) return false;
    }
    for (var index = 0; index < offsets.length; index++) {
      if (offsets[index] != other.offsets[index]) return false;
    }
    return true;
  }
}

class GalleryEntryIdentities {
  GalleryEntryIdentities(List<GalleryEntryIdentityBlock> blocks)
    : blocks = List.unmodifiable(blocks) {
    starts = Uint32List(blocks.length + 1);
    for (var index = 0; index < blocks.length; index++) {
      starts[index + 1] = starts[index] + blocks[index].length;
    }
  }

  final List<GalleryEntryIdentityBlock> blocks;
  late final Uint32List starts;

  int get byteLength =>
      starts.lengthInBytes +
      blocks.fold(0, (sum, block) => sum + block.byteLength);

  String at(int index) {
    var lower = 0;
    var upper = blocks.length;
    while (lower < upper) {
      final middle = lower + ((upper - lower) >> 1);
      if (starts[middle + 1] <= index) {
        lower = middle + 1;
      } else {
        upper = middle;
      }
    }
    return blocks[lower].at(index - starts[lower]);
  }

  bool sameAs(GalleryEntryIdentities other) {
    if (blocks.length != other.blocks.length) return false;
    for (var index = 0; index < blocks.length; index++) {
      if (!blocks[index].sameAs(other.blocks[index])) return false;
    }
    return true;
  }
}
