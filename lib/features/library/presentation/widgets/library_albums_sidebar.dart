import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "library_gallery_message.dart";

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
    child: AmeScrollMotion(
      child: ListView(
        padding: const EdgeInsets.fromLTRB(8, 12, 8, 12),
        children: [child],
      ),
    ),
  );
}

class LibraryAlbumsLanding extends StatelessWidget {
  const LibraryAlbumsLanding({super.key});

  @override
  Widget build(BuildContext context) => const LibraryGalleryMessage(
    icon: Symbols.photo_album_rounded,
    title: "选择相册",
    detail: "在侧栏中选择收藏夹或相册，也可以新建相册。",
  );
}
