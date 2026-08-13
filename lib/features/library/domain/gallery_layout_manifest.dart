import "dart:convert";
import "dart:typed_data";

import "library_models.dart";

const libraryGalleryLayoutDimensionsKnownFlag = 1;

enum LibraryGalleryLayoutManifestStorageKind { flat, hierarchical }

class LibraryGalleryLayoutManifestStoragePlan {
  const LibraryGalleryLayoutManifestStoragePlan({
    required this.kind,
    required this.estimatedBytes,
  });

  static const flatBudgetBytes = 64 * 1024 * 1024;
  static const maximumEncodedLocationIdBytes = 128;
  static const fixedBytesPerItem =
      maximumEncodedLocationIdBytes +
      Uint32List.bytesPerElement +
      Uint16List.bytesPerElement +
      Uint32List.bytesPerElement +
      Uint8List.bytesPerElement;

  factory LibraryGalleryLayoutManifestStoragePlan.forItemCount(int itemCount) {
    if (itemCount < 0) {
      throw const LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_count_invalid",
        message: "The gallery layout item count cannot be negative",
      );
    }
    final estimatedBytes =
        itemCount * fixedBytesPerItem + Uint32List.bytesPerElement;
    return LibraryGalleryLayoutManifestStoragePlan(
      kind: estimatedBytes <= flatBudgetBytes
          ? LibraryGalleryLayoutManifestStorageKind.flat
          : LibraryGalleryLayoutManifestStorageKind.hierarchical,
      estimatedBytes: estimatedBytes,
    );
  }

  final LibraryGalleryLayoutManifestStorageKind kind;
  final int estimatedBytes;
}

class LibraryGalleryLayoutManifestCursor {
  const LibraryGalleryLayoutManifestCursor({
    required this.revision,
    required this.queryId,
    required this.totalItems,
    required this.nextOrdinal,
    required this.after,
  });

  final BigInt revision;
  final String queryId;
  final int totalItems;
  final int nextOrdinal;
  final LibraryCatalogCursor after;
}

class LibraryGalleryLayoutManifestChunk {
  LibraryGalleryLayoutManifestChunk({
    required this.revision,
    required this.queryId,
    required this.totalItems,
    required this.startOrdinal,
    required List<String> locationIds,
    required Uint16List aspectRatioMilli,
    required Uint16List dateGroupIndices,
    required List<String?> dateGroups,
    required Uint8List flags,
    this.nextCursor,
  }) : locationIds = List.unmodifiable(locationIds),
       aspectRatioMilli = Uint16List.fromList(aspectRatioMilli),
       dateGroupIndices = Uint16List.fromList(dateGroupIndices),
       dateGroups = List.unmodifiable(dateGroups),
       flags = Uint8List.fromList(flags);

  final BigInt revision;
  final String queryId;
  final int totalItems;
  final int startOrdinal;
  final List<String> locationIds;
  final Uint16List aspectRatioMilli;
  final Uint16List dateGroupIndices;
  final List<String?> dateGroups;
  final Uint8List flags;
  final LibraryGalleryLayoutManifestCursor? nextCursor;

  int get itemCount => locationIds.length;
}

class LibraryGalleryLayoutManifest {
  const LibraryGalleryLayoutManifest._({
    required this.revision,
    required this.queryId,
    required this._storage,
    this._dimensionOverrides = const {},
  });

  final BigInt revision;
  final String queryId;
  final _LibraryGalleryLayoutManifestStorage _storage;
  final Map<int, int> _dimensionOverrides;

  int get itemCount => _storage.itemCount;

  int get primitiveByteLength =>
      _storage.primitiveByteLength + _dimensionOverrides.length * 8;

  LibraryGalleryLayoutManifestStorageKind get storageKind => _storage.kind;

  String locationIdAt(int index) {
    _checkIndex(index);
    return _storage.locationIdAt(index);
  }

  double aspectRatioAt(int index) {
    _checkIndex(index);
    return (_dimensionOverrides[index] ??
            (_storage.aspectRatioAt(index) * 1000).round()) /
        1000;
  }

  String? dateKeyAt(int index) {
    _checkIndex(index);
    return _storage.dateKeyAt(index);
  }

  bool hasKnownDimensionsAt(int index) {
    _checkIndex(index);
    return _dimensionOverrides.containsKey(index) ||
        _storage.hasKnownDimensionsAt(index);
  }

  LibraryGalleryLayoutManifest withDimensionUpdates(
    Iterable<LibraryGalleryLayoutDimensionUpdate> updates,
  ) {
    Map<int, int>? nextOverrides;
    for (final update in updates) {
      if (update.revision != revision ||
          update.queryId != queryId ||
          update.globalItemIndex < 0 ||
          update.globalItemIndex >= itemCount ||
          update.width <= 0 ||
          update.height <= 0 ||
          locationIdAt(update.globalItemIndex) != update.locationId) {
        continue;
      }
      final aspectRatioMilli = ((update.width * 1000) ~/ update.height).clamp(
        200,
        5000,
      );
      if (hasKnownDimensionsAt(update.globalItemIndex) &&
          (aspectRatioAt(update.globalItemIndex) * 1000).round() ==
              aspectRatioMilli) {
        continue;
      }
      nextOverrides ??= Map<int, int>.of(_dimensionOverrides);
      nextOverrides[update.globalItemIndex] = aspectRatioMilli;
    }
    if (nextOverrides == null) {
      return this;
    }
    return LibraryGalleryLayoutManifest._(
      revision: revision,
      queryId: queryId,
      storage: _storage,
      dimensionOverrides: Map.unmodifiable(nextOverrides),
    );
  }

  void _checkIndex(int index) {
    if (index < 0 || index >= itemCount) {
      throw RangeError.index(index, this, "index", null, itemCount);
    }
  }
}

class LibraryGalleryLayoutDimensionUpdate {
  const LibraryGalleryLayoutDimensionUpdate({
    required this.revision,
    required this.queryId,
    required this.globalItemIndex,
    required this.locationId,
    required this.width,
    required this.height,
  });

  final BigInt revision;
  final String queryId;
  final int globalItemIndex;
  final String locationId;
  final int width;
  final int height;
}

abstract class LibraryGalleryLayoutManifestBuilder {
  factory LibraryGalleryLayoutManifestBuilder({
    required BigInt revision,
    required String queryId,
    required int totalItems,
  }) {
    final plan = LibraryGalleryLayoutManifestStoragePlan.forItemCount(
      totalItems,
    );
    return switch (plan.kind) {
      LibraryGalleryLayoutManifestStorageKind.flat =>
        _FlatLibraryGalleryLayoutManifestBuilder(
          revision: revision,
          queryId: queryId,
          totalItems: totalItems,
        ),
      LibraryGalleryLayoutManifestStorageKind.hierarchical =>
        _HierarchicalLibraryGalleryLayoutManifestBuilder(
          revision: revision,
          queryId: queryId,
          totalItems: totalItems,
        ),
    };
  }

  void append(LibraryGalleryLayoutManifestChunk chunk);

  LibraryGalleryLayoutManifest build();

  LibraryGalleryLayoutManifestStorageKind get storageKind;
}

abstract class _LibraryGalleryLayoutManifestBuilderBase
    implements LibraryGalleryLayoutManifestBuilder {
  _LibraryGalleryLayoutManifestBuilderBase({
    required this.revision,
    required this.queryId,
    required this.totalItems,
  });

  final BigInt revision;
  final String queryId;
  final int totalItems;
  var itemCount = 0;
  var isBuilt = false;

  void ensureCanAppend(LibraryGalleryLayoutManifestChunk chunk) {
    if (isBuilt) {
      throw const LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_already_built",
        message: "The gallery layout manifest builder is already complete",
      );
    }
    validateChunkHeader(chunk);
    validateChunkColumns(chunk);
  }

  void ensureCanBuild() {
    if (isBuilt) {
      throw const LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_already_built",
        message: "The gallery layout manifest builder is already complete",
      );
    }
    if (itemCount != totalItems) {
      throw LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_incomplete",
        message:
            "The gallery layout manifest contains $itemCount of "
            "$totalItems items",
      );
    }
    isBuilt = true;
  }

  void validateChunkHeader(LibraryGalleryLayoutManifestChunk chunk) {
    if (chunk.revision != revision ||
        chunk.queryId != queryId ||
        chunk.totalItems != totalItems ||
        chunk.startOrdinal != itemCount) {
      throw const LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_chunk_mismatch",
        message: "A gallery layout chunk does not continue the active manifest",
      );
    }
    final endOrdinal = itemCount + chunk.itemCount;
    if (endOrdinal > totalItems) {
      throw const LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_count_invalid",
        message: "A gallery layout chunk exceeds the complete query count",
      );
    }
    final cursor = chunk.nextCursor;
    if (chunk.itemCount == 0 && cursor != null) {
      throw const LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_cursor_invalid",
        message: "A non-terminal gallery layout chunk cannot be empty",
      );
    }
    if (cursor == null && endOrdinal != totalItems) {
      throw const LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_incomplete",
        message: "A non-terminal gallery layout chunk requires a cursor",
      );
    }
    if (cursor != null &&
        (cursor.revision != revision ||
            cursor.queryId != queryId ||
            cursor.totalItems != totalItems ||
            cursor.nextOrdinal != endOrdinal ||
            endOrdinal >= totalItems)) {
      throw const LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_cursor_invalid",
        message: "A gallery layout chunk contains an incompatible cursor",
      );
    }
  }

  void validateChunkColumns(LibraryGalleryLayoutManifestChunk chunk) {
    final chunkItemCount = chunk.itemCount;
    if (chunk.aspectRatioMilli.length != chunkItemCount ||
        chunk.dateGroupIndices.length != chunkItemCount ||
        chunk.flags.length != chunkItemCount) {
      throw const LibraryGalleryLayoutManifestFailure(
        code: "gallery_layout_manifest_columns_invalid",
        message: "Gallery layout chunk columns have different lengths",
      );
    }
  }
}

class _FlatLibraryGalleryLayoutManifestBuilder
    extends _LibraryGalleryLayoutManifestBuilderBase {
  _FlatLibraryGalleryLayoutManifestBuilder({
    required super.revision,
    required super.queryId,
    required super.totalItems,
  }) : _locationIdOffsets = Uint32List(totalItems + 1),
       _aspectRatioMilli = Uint16List(totalItems),
       _dateGroupIndices = Uint32List(totalItems),
       _flags = Uint8List(totalItems);

  final BytesBuilder _locationIdBytes = BytesBuilder(copy: false);
  final Uint32List _locationIdOffsets;
  final Uint16List _aspectRatioMilli;
  final Uint32List _dateGroupIndices;
  final Uint8List _flags;
  final List<String?> _dateGroups = [];
  final Map<String?, int> _dateGroupLookup = {};
  var _encodedLocationIdBytes = 0;

  @override
  LibraryGalleryLayoutManifestStorageKind get storageKind =>
      LibraryGalleryLayoutManifestStorageKind.flat;

  @override
  void append(LibraryGalleryLayoutManifestChunk chunk) {
    ensureCanAppend(chunk);
    for (var localIndex = 0; localIndex < chunk.itemCount; localIndex++) {
      final globalIndex = itemCount + localIndex;
      final encodedLocationId = utf8.encode(chunk.locationIds[localIndex]);
      if (encodedLocationId.isEmpty) {
        throw const LibraryGalleryLayoutManifestFailure(
          code: "gallery_layout_manifest_identity_invalid",
          message: "A gallery layout location identity cannot be empty",
        );
      }
      _locationIdBytes.add(encodedLocationId);
      _encodedLocationIdBytes += encodedLocationId.length;
      if (_encodedLocationIdBytes > 0xffffffff) {
        throw const LibraryGalleryLayoutManifestFailure(
          code: "gallery_layout_manifest_identity_bytes_invalid",
          message: "Gallery layout location identities exceed the flat range",
        );
      }
      _locationIdOffsets[globalIndex + 1] = _encodedLocationIdBytes;
      _aspectRatioMilli[globalIndex] = chunk.aspectRatioMilli[localIndex];
      _flags[globalIndex] = chunk.flags[localIndex];

      final localDateIndex = chunk.dateGroupIndices[localIndex];
      if (localDateIndex >= chunk.dateGroups.length) {
        throw const LibraryGalleryLayoutManifestFailure(
          code: "gallery_layout_manifest_date_group_invalid",
          message: "A gallery layout item references an absent date group",
        );
      }
      final dateKey = chunk.dateGroups[localDateIndex];
      final globalDateIndex = _dateGroupLookup.putIfAbsent(dateKey, () {
        final index = _dateGroups.length;
        _dateGroups.add(dateKey);
        return index;
      });
      _dateGroupIndices[globalIndex] = globalDateIndex;
    }
    itemCount += chunk.itemCount;
  }

  @override
  LibraryGalleryLayoutManifest build() {
    ensureCanBuild();
    return LibraryGalleryLayoutManifest._(
      revision: revision,
      queryId: queryId,
      storage: _FlatLibraryGalleryLayoutManifestStorage(
        itemCount: totalItems,
        locationIdBytes: _locationIdBytes.takeBytes(),
        locationIdOffsets: _locationIdOffsets,
        aspectRatioMilli: _aspectRatioMilli,
        dateGroupIndices: _dateGroupIndices,
        dateGroups: List<String?>.unmodifiable(_dateGroups),
        flags: _flags,
      ),
    );
  }
}

class _HierarchicalLibraryGalleryLayoutManifestBuilder
    extends _LibraryGalleryLayoutManifestBuilderBase {
  _HierarchicalLibraryGalleryLayoutManifestBuilder({
    required super.revision,
    required super.queryId,
    required super.totalItems,
  });

  final List<_LibraryGalleryLayoutManifestBlock> _blocks = [];

  @override
  LibraryGalleryLayoutManifestStorageKind get storageKind =>
      LibraryGalleryLayoutManifestStorageKind.hierarchical;

  @override
  void append(LibraryGalleryLayoutManifestChunk chunk) {
    ensureCanAppend(chunk);
    if (chunk.itemCount > 0) {
      _blocks.add(_LibraryGalleryLayoutManifestBlock.fromChunk(chunk));
    }
    itemCount += chunk.itemCount;
  }

  @override
  LibraryGalleryLayoutManifest build() {
    ensureCanBuild();
    return LibraryGalleryLayoutManifest._(
      revision: revision,
      queryId: queryId,
      storage: _HierarchicalLibraryGalleryLayoutManifestStorage(
        itemCount: totalItems,
        blocks: List.unmodifiable(_blocks),
      ),
    );
  }
}

abstract interface class _LibraryGalleryLayoutManifestStorage {
  LibraryGalleryLayoutManifestStorageKind get kind;

  int get itemCount;

  int get primitiveByteLength;

  String locationIdAt(int index);

  double aspectRatioAt(int index);

  String? dateKeyAt(int index);

  bool hasKnownDimensionsAt(int index);
}

class _FlatLibraryGalleryLayoutManifestStorage
    implements _LibraryGalleryLayoutManifestStorage {
  const _FlatLibraryGalleryLayoutManifestStorage({
    required this.itemCount,
    required this._locationIdBytes,
    required this._locationIdOffsets,
    required this._aspectRatioMilli,
    required this._dateGroupIndices,
    required this._dateGroups,
    required this._flags,
  });

  @override
  LibraryGalleryLayoutManifestStorageKind get kind =>
      LibraryGalleryLayoutManifestStorageKind.flat;

  @override
  final int itemCount;
  final Uint8List _locationIdBytes;
  final Uint32List _locationIdOffsets;
  final Uint16List _aspectRatioMilli;
  final Uint32List _dateGroupIndices;
  final List<String?> _dateGroups;
  final Uint8List _flags;

  @override
  int get primitiveByteLength =>
      _locationIdBytes.lengthInBytes +
      _locationIdOffsets.lengthInBytes +
      _aspectRatioMilli.lengthInBytes +
      _dateGroupIndices.lengthInBytes +
      _flags.lengthInBytes +
      _encodedDateBytes(_dateGroups);

  @override
  String locationIdAt(int index) => utf8.decode(
    Uint8List.sublistView(
      _locationIdBytes,
      _locationIdOffsets[index],
      _locationIdOffsets[index + 1],
    ),
  );

  @override
  double aspectRatioAt(int index) => _aspectRatioMilli[index] / 1000;

  @override
  String? dateKeyAt(int index) => _dateGroups[_dateGroupIndices[index]];

  @override
  bool hasKnownDimensionsAt(int index) =>
      _flags[index] & libraryGalleryLayoutDimensionsKnownFlag != 0;
}

class _HierarchicalLibraryGalleryLayoutManifestStorage
    implements _LibraryGalleryLayoutManifestStorage {
  _HierarchicalLibraryGalleryLayoutManifestStorage({
    required this.itemCount,
    required this.blocks,
  }) : _blockStarts = Uint32List.fromList([
         for (final block in blocks) block.startOrdinal,
       ]);

  @override
  LibraryGalleryLayoutManifestStorageKind get kind =>
      LibraryGalleryLayoutManifestStorageKind.hierarchical;

  @override
  final int itemCount;
  final List<_LibraryGalleryLayoutManifestBlock> blocks;
  final Uint32List _blockStarts;

  @override
  int get primitiveByteLength =>
      _blockStarts.lengthInBytes +
      blocks.fold(0, (total, block) => total + block.primitiveByteLength);

  @override
  String locationIdAt(int index) {
    final block = _blockFor(index);
    return block.locationIdAt(index - block.startOrdinal);
  }

  @override
  double aspectRatioAt(int index) {
    final block = _blockFor(index);
    return block.aspectRatioAt(index - block.startOrdinal);
  }

  @override
  String? dateKeyAt(int index) {
    final block = _blockFor(index);
    return block.dateKeyAt(index - block.startOrdinal);
  }

  @override
  bool hasKnownDimensionsAt(int index) {
    final block = _blockFor(index);
    return block.hasKnownDimensionsAt(index - block.startOrdinal);
  }

  _LibraryGalleryLayoutManifestBlock _blockFor(int index) {
    var lower = 0;
    var upper = _blockStarts.length;
    while (lower < upper) {
      final middle = lower + ((upper - lower) >> 1);
      if (_blockStarts[middle] <= index) {
        lower = middle + 1;
      } else {
        upper = middle;
      }
    }
    return blocks[lower - 1];
  }
}

class _LibraryGalleryLayoutManifestBlock {
  _LibraryGalleryLayoutManifestBlock({
    required this.startOrdinal,
    required this._locationIdBytes,
    required this._locationIdOffsets,
    required this._aspectRatioMilli,
    required this._dateGroupIndices,
    required this._dateGroups,
    required this._flags,
  });

  factory _LibraryGalleryLayoutManifestBlock.fromChunk(
    LibraryGalleryLayoutManifestChunk chunk,
  ) {
    final bytes = BytesBuilder(copy: false);
    final offsets = Uint32List(chunk.itemCount + 1);
    var encodedBytes = 0;
    for (var index = 0; index < chunk.itemCount; index++) {
      final encoded = utf8.encode(chunk.locationIds[index]);
      if (encoded.isEmpty) {
        throw const LibraryGalleryLayoutManifestFailure(
          code: "gallery_layout_manifest_identity_invalid",
          message: "A gallery layout location identity cannot be empty",
        );
      }
      bytes.add(encoded);
      encodedBytes += encoded.length;
      offsets[index + 1] = encodedBytes;
      if (chunk.dateGroupIndices[index] >= chunk.dateGroups.length) {
        throw const LibraryGalleryLayoutManifestFailure(
          code: "gallery_layout_manifest_date_group_invalid",
          message: "A gallery layout item references an absent date group",
        );
      }
    }
    return _LibraryGalleryLayoutManifestBlock(
      startOrdinal: chunk.startOrdinal,
      locationIdBytes: bytes.takeBytes(),
      locationIdOffsets: offsets,
      aspectRatioMilli: Uint16List.fromList(chunk.aspectRatioMilli),
      dateGroupIndices: Uint16List.fromList(chunk.dateGroupIndices),
      dateGroups: List<String?>.unmodifiable(chunk.dateGroups),
      flags: Uint8List.fromList(chunk.flags),
    );
  }

  final int startOrdinal;
  final Uint8List _locationIdBytes;
  final Uint32List _locationIdOffsets;
  final Uint16List _aspectRatioMilli;
  final Uint16List _dateGroupIndices;
  final List<String?> _dateGroups;
  final Uint8List _flags;

  int get primitiveByteLength =>
      _locationIdBytes.lengthInBytes +
      _locationIdOffsets.lengthInBytes +
      _aspectRatioMilli.lengthInBytes +
      _dateGroupIndices.lengthInBytes +
      _flags.lengthInBytes +
      _encodedDateBytes(_dateGroups);

  String locationIdAt(int index) => utf8.decode(
    Uint8List.sublistView(
      _locationIdBytes,
      _locationIdOffsets[index],
      _locationIdOffsets[index + 1],
    ),
  );

  double aspectRatioAt(int index) => _aspectRatioMilli[index] / 1000;

  String? dateKeyAt(int index) => _dateGroups[_dateGroupIndices[index]];

  bool hasKnownDimensionsAt(int index) =>
      _flags[index] & libraryGalleryLayoutDimensionsKnownFlag != 0;
}

int _encodedDateBytes(List<String?> dates) {
  return dates.fold<int>(
    0,
    (total, date) => total + (date == null ? 0 : utf8.encode(date).length),
  );
}

class LibraryGalleryLayoutManifestFailure implements Exception {
  const LibraryGalleryLayoutManifestFailure({
    required this.code,
    required this.message,
  });

  final String code;
  final String message;

  @override
  String toString() => "$code: $message";
}
