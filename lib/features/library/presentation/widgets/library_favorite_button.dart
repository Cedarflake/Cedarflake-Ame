import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_organization_actions.dart";
import "../../domain/library_organization.dart";

class LibraryFavoriteButton extends ConsumerWidget {
  const LibraryFavoriteButton({required this.target, super.key});
  final LibraryOrganizationTarget target;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final status = ref.watch(libraryOrganizationInspectionProvider(target));
    final saving = ref.watch(
      libraryOrganizationActionsProvider.select((value) => value.saving),
    );
    final favorite = status.value?.decisions.singleOrNull?.favorite;
    final failed = status.hasError;
    final label = _label(
      favorite: favorite,
      saving: saving,
      error: status.error,
    );
    VoidCallback? action;
    if (!saving && !status.isLoading) {
      if (failed) {
        action = () =>
            ref.invalidate(libraryOrganizationInspectionProvider(target));
      } else if (favorite != null) {
        action = () => unawaited(
          ref
              .read(libraryOrganizationActionsProvider.notifier)
              .favorite(target, !favorite),
        );
      }
    }
    return AmeTooltip(
      message: label,
      child: IconButton(
        key: const Key("viewer-favorite"),
        iconSize: 20,
        isSelected: failed ? false : favorite ?? false,
        onPressed: action,
        icon: Icon(
          failed ? Symbols.refresh_rounded : Symbols.favorite_rounded,
          opticalSize: 20,
          weight: 400,
          fill: 0,
          semanticLabel: label,
        ),
        selectedIcon: Icon(
          Symbols.favorite_rounded,
          opticalSize: 20,
          weight: 400,
          fill: 1,
          semanticLabel: label,
        ),
      ),
    );
  }

  String _label({
    required bool? favorite,
    required bool saving,
    required Object? error,
  }) {
    if (error is LibraryOrganizationFailure &&
        error.code == "organization_evidence_pending") {
      return "正在确认图片内容，点击重试";
    }
    if (error != null) {
      return "无法读取收藏状态，重试";
    }
    if (saving) {
      return "正在保存更改";
    }
    if (favorite == null) {
      return "正在读取收藏状态";
    }
    return favorite ? "取消收藏" : "加入收藏夹";
  }
}
