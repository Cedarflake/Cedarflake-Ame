import "package:flutter/material.dart";

import "../../application/library_controller.dart";
import "../../application/library_sidebar_root_removal.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "library_sidebar_removal_dialog.dart";

Future<Set<String>> showLibrarySidebarRootRemovalDialog({
  required BuildContext context,
  required List<LibraryRoot> roots,
  required LibraryController controller,
  required LibraryState Function() readState,
}) => showLibrarySidebarRemovalDialog(
  context: context,
  removal: createSidebarRootRemoval(
    roots: roots,
    controller: controller,
    readState: readState,
  ),
  title: "从 Ame 中移除 ${roots.length} 个文件夹？",
  detail:
      "这些文件夹中的图片将不再显示在 Ame 中，仅属于这些文件夹的相册归属、收藏和整理记录也会移除。其他来源仍有的重复图片会保留记录。原目录和图片文件都会保留。",
  actionLabel: "从 Ame 中移除",
);
