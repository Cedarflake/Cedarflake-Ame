import "dart:typed_data";

/// Compares encoded gallery columns exactly, without copying their buffers.
bool galleryBuffersEqual(TypedData left, TypedData right) {
  if (identical(left, right)) return true;
  if (left.lengthInBytes != right.lengthInBytes) return false;
  final leftBytes = ByteData.view(
    left.buffer,
    left.offsetInBytes,
    left.lengthInBytes,
  );
  final rightBytes = ByteData.view(
    right.buffer,
    right.offsetInBytes,
    right.lengthInBytes,
  );
  final wordEnd = left.lengthInBytes - left.lengthInBytes % 8;
  for (var offset = 0; offset < wordEnd; offset += 8) {
    if (leftBytes.getUint64(offset, Endian.host) !=
        rightBytes.getUint64(offset, Endian.host)) {
      return false;
    }
  }
  for (var offset = wordEnd; offset < left.lengthInBytes; offset++) {
    if (leftBytes.getUint8(offset) != rightBytes.getUint8(offset)) return false;
  }
  return true;
}
