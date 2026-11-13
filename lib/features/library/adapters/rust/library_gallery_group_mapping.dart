import "../../../../src/rust/domain/gallery_groups.dart" as rust;
import "../../domain/library_gallery_group.dart";

LibraryGalleryGroupKey? mapRustGalleryGroupKey(rust.GalleryGroupKey? key) =>
    switch (key) {
      null => null,
      rust.GalleryGroupKey_Exact(:final digest, :final byteLength) =>
        LibraryExactGalleryGroupKey(digest, byteLength),
      rust.GalleryGroupKey_Similar(:final groupId) =>
        LibrarySimilarGalleryGroupKey(groupId),
    };

rust.GalleryGroupKey? mapGalleryGroupKeyToRust(LibraryGalleryGroupKey? key) =>
    switch (key) {
      null => null,
      LibraryExactGalleryGroupKey(:final digest, :final byteLength) =>
        rust.GalleryGroupKey.exact(digest: digest, byteLength: byteLength),
      LibrarySimilarGalleryGroupKey(:final groupId) =>
        rust.GalleryGroupKey.similar(groupId: groupId),
    };

LibraryGalleryGroup? mapRustGalleryGroup(rust.GalleryGroup? group) {
  if (group == null) return null;
  return LibraryGalleryGroup(
    key: mapRustGalleryGroupKey(group.key)!,
    memberCount: group.memberCount.toInt(),
  );
}

LibraryGalleryGroupFilter mapRustGalleryGroupFilter(
  rust.GalleryGroupFilter filter,
) => switch (filter) {
  rust.GalleryGroupFilter.unrestricted =>
    LibraryGalleryGroupFilter.unrestricted,
  rust.GalleryGroupFilter.exact => LibraryGalleryGroupFilter.exact,
  rust.GalleryGroupFilter.similar => LibraryGalleryGroupFilter.similar,
};

rust.GalleryGroupFilter mapGalleryGroupFilterToRust(
  LibraryGalleryGroupFilter filter,
) => switch (filter) {
  LibraryGalleryGroupFilter.unrestricted =>
    rust.GalleryGroupFilter.unrestricted,
  LibraryGalleryGroupFilter.exact => rust.GalleryGroupFilter.exact,
  LibraryGalleryGroupFilter.similar => rust.GalleryGroupFilter.similar,
};
