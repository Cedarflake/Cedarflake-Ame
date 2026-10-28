import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_menu.dart";
import "../../application/library_album_requests.dart";
import "../../application/library_organization.dart";
import "../../application/library_organization_actions.dart";
import "../../domain/library_organization.dart";
import "../library_strings.dart";
import "library_album_picker.dart";

class LibraryPhotoContextMenu {
  bool _isOpen = false;
  bool _disposed = false;
  LibraryAlbumMembership? _membership;

  void dispose() {
    _disposed = true;
    _membership?.dispose();
  }

  Future<void> open({
    required BuildContext context,
    required WidgetRef ref,
    required RelativeRect position,
    required LibraryOrganizationTarget target,
    required bool Function() isCurrent,
    required VoidCallback onOpen,
    required VoidCallback onInformation,
    required VoidCallback onCopyPath,
    required VoidCallback onReveal,
  }) async {
    if (_disposed || _isOpen) return;
    _isOpen = true;
    bool owns() => !_disposed && context.mounted && isCurrent();
    try {
      LibraryOrganizationDecision? decision;
      try {
        final inspection = await ref.read(libraryOrganizationProvider).inspect([
          target,
        ]);
        decision = inspection.decisions.single;
      } on Object {
        if (!owns()) return;
        ref
            .read(ameNotificationControllerProvider.notifier)
            .publish(
              const AmeNotificationDraft(
                title: "暂时无法读取相册和收藏",
                message: "请重新打开菜单重试。",
                severity: AmeNotificationSeverity.error,
                dedupeKey: "photo-menu-inspection",
              ),
            );
      }
      if (!context.mounted || !owns()) return;
      if (decision != null) {
        ref
            .read(ameNotificationControllerProvider.notifier)
            .resolve("photo-menu-inspection");
      }
      final busy = ref.read(libraryOrganizationActionsProvider).saving;
      final entries = _entries(decision);
      final action = await showAmePopupMenu<_PhotoAction>(
        context: context,
        position: position,
        labels: [for (final entry in entries) entry.label],
        items: [
          for (final entry in entries) ...[
            if (entry.action == _PhotoAction.copyPath)
              const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
            PopupMenuItem(
              value: entry.action,
              enabled: !busy || !entry.action.changesOrganization,
              child: AmeMenuItemContent(icon: entry.icon, label: entry.label),
            ),
          ],
        ],
      );
      if (!context.mounted || !owns() || action == null) return;
      final commands = ref.read(libraryOrganizationActionsProvider.notifier);
      switch (action) {
        case _PhotoAction.open:
          onOpen();
        case _PhotoAction.information:
          onInformation();
        case _PhotoAction.copyPath:
          onCopyPath();
        case _PhotoAction.reveal:
          onReveal();
        case _PhotoAction.favorite:
          await commands.favorite(target, !decision!.favorite);
        case _PhotoAction.removeAlbum:
          await commands.submit(LibraryAlbumRemovalRequest(target));
        case _PhotoAction.album:
          final current = decision!.album;
          final membership = LibraryAlbumMembershipSession([
            target,
          ], isMove: current != null);
          _membership = membership;
          await showLibraryAlbumPicker(
            context: context,
            actionLabel: current == null ? "加入相册" : "移至其他相册",
            actionIcon: current == null
                ? Symbols.bookmark_add_rounded
                : Symbols.drive_file_move_rounded,
            initialAlbum: current,
            currentAlbum: current,
            clearLabel: current == null ? null : "移出相册",
            explanation: decision.hasConflict ? "这组图片的相册归属有冲突，请选择一个相册。" : null,
            conflictTarget: decision.hasConflict ? target : null,
            membership: membership,
          );
      }
    } finally {
      _membership?.dispose();
      _membership = null;
      _isOpen = false;
    }
  }

  List<_PhotoEntry> _entries(LibraryOrganizationDecision? decision) => [
    const _PhotoEntry(
      _PhotoAction.open,
      LibraryStrings.open,
      Symbols.open_in_full_rounded,
    ),
    if (decision != null) ...[
      _PhotoEntry(
        _PhotoAction.favorite,
        decision.favorite ? "取消收藏" : "加入收藏夹",
        decision.favorite
            ? Symbols.heart_minus_rounded
            : Symbols.favorite_rounded,
      ),
      _PhotoEntry(
        _PhotoAction.album,
        _albumLabel(decision),
        decision.album == null
            ? Symbols.bookmark_add_rounded
            : Symbols.drive_file_move_rounded,
      ),
      if (decision.album != null)
        const _PhotoEntry(
          _PhotoAction.removeAlbum,
          "移出相册",
          Symbols.bookmark_remove_rounded,
        ),
    ],
    const _PhotoEntry(
      _PhotoAction.information,
      LibraryStrings.viewInformation,
      Symbols.info_rounded,
    ),
    const _PhotoEntry(
      _PhotoAction.copyPath,
      LibraryStrings.copyPath,
      Symbols.content_copy_rounded,
    ),
    const _PhotoEntry(
      _PhotoAction.reveal,
      LibraryStrings.openInExplorer,
      Symbols.folder_open_rounded,
    ),
  ];

  String _albumLabel(LibraryOrganizationDecision decision) {
    if (decision.hasConflict) return "选择相册";
    if (decision.album != null) return "移至其他相册";
    return "加入相册";
  }
}

enum _PhotoAction {
  open,
  favorite,
  album,
  removeAlbum,
  information,
  copyPath,
  reveal;

  bool get changesOrganization => switch (this) {
    favorite || album || removeAlbum => true,
    _ => false,
  };
}

class _PhotoEntry {
  const _PhotoEntry(this.action, this.label, this.icon);
  final _PhotoAction action;
  final String label;
  final IconData icon;
}
