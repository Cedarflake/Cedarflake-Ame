import "package:material_ui/material_ui.dart";
import "package:material_symbols_icons/symbols.dart";

import "library_gallery_message.dart";
import "library_gallery_loading_region.dart";

class LibraryAlbumsSidebar extends StatelessWidget {
  const LibraryAlbumsSidebar({
    required this.width,
    required this.child,
    super.key,
  });

  final double width;
  final Widget child;

  @override
  Widget build(BuildContext context) => SizedBox(
    key: const Key("library-albums-navigation"),
    width: width,
    child: child,
  );
}

class LibraryAlbumsLanding extends StatelessWidget {
  const LibraryAlbumsLanding({this.isLoading = false, super.key});

  final bool isLoading;

  @override
  Widget build(BuildContext context) => LibraryGalleryLoadingRegion(
    isLoading: isLoading,
    delay: const Duration(milliseconds: 250),
    child: const LibraryGalleryMessage(
      icon: Symbols.photo_album_rounded,
      title: "选择相册",
      detail: "在侧栏中选择收藏夹或相册，也可以新建相册。",
    ),
  );
}
