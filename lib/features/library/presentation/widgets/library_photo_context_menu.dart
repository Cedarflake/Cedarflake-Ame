import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/messages/ame_message.dart";
import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_menu.dart";
import "../../application/library_album_requests.dart";
import "../../application/library_controller.dart";
import "../../application/library_file_operations.dart";
import "../../application/library_file_selection_scope.dart";
import "../../domain/library_file_operations.dart";
import "../../application/library_organization.dart";
import "../../application/library_organization_actions.dart";
import "../../domain/library_organization.dart";
import "../../domain/library_organization_batch.dart";
import "../../domain/library_gallery_group.dart";
import "library_album_picker.dart";
import "library_file_action_dialog.dart";

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
    LibraryGalleryGroupKey? groupKey,
    String? fileName,
    VoidCallback? onFindRelated,
  }) async {
    if (_disposed || _isOpen) return;
    _isOpen = true;
    final library = ref.read(libraryControllerProvider);
    final query = library.query;
    final catalogRevision = library.catalogRevision;
    bool owns() =>
        !_disposed &&
        context.mounted &&
        isCurrent() &&
        ref.read(libraryControllerProvider).query == query;
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
                title: AmeLabel.albumsAndFavoritesUnavailable,
                message: AmeLabel.reopenMenuRetry,
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
      final filesBusy =
          ref.read(libraryFileActionsProvider).busy || catalogRevision == null;
      final entries = _entries(context.l10n, decision, onFindRelated != null);
      final action = await showAmePopupMenu<_PhotoAction>(
        context: context,
        position: position,
        labels: [for (final entry in entries) entry.label],
        items: [
          for (final entry in entries) ...[
            if (entry.action == _PhotoAction.copyPath ||
                entry.action == _PhotoAction.copyFile)
              const PopupMenuDivider(height: AmeMenuMetrics.dividerHeight),
            PopupMenuItem(
              value: entry.action,
              enabled:
                  (!busy || !entry.action.changesOrganization) &&
                  (!filesBusy || !entry.action.changesFiles),
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
        case _PhotoAction.related:
          onFindRelated?.call();
        case _PhotoAction.copyFile:
        case _PhotoAction.moveFile:
        case _PhotoAction.renameFile:
        case _PhotoAction.deleteFile:
          final currentRevision = ref
              .read(libraryControllerProvider)
              .catalogRevision;
          if (currentRevision == null) return;
          final fileAction = switch (action) {
            _PhotoAction.copyFile => LibraryFileAction.copy,
            _PhotoAction.moveFile => LibraryFileAction.move,
            _PhotoAction.renameFile => LibraryFileAction.rename,
            _ => LibraryFileAction.recycle,
          };
          await showLibraryFileAction(
            context: context,
            ref: ref,
            selection: selectGalleryFilesForAction(
              action: fileAction,
              selection: LibraryOrganizationSelection(
                query: query,
                catalogRevision: currentRevision,
                allMatching: false,
                assetIds: [groupKey?.entryId ?? target.assetId],
              ),
              location:
                  groupKey != null && fileAction == LibraryFileAction.recycle
                  ? null
                  : target,
            ),
            initialName: fileName,
            action: fileAction,
            initialRule: initialFileOrganizationRule(
              action: fileAction,
              query: query,
            ),
          );
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
            actionLabel: current == null
                ? context.l10n.addToAlbum
                : context.l10n.moveToAnotherAlbum,
            actionIcon: current == null
                ? Symbols.bookmark_add_rounded
                : Symbols.drive_file_move_rounded,
            initialAlbum: current,
            currentAlbum: current,
            clearLabel: current == null ? null : context.l10n.removeFromAlbum,
            explanation: decision.hasConflict
                ? context.l10n.albumGroupConflict
                : null,
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

  List<_PhotoEntry> _entries(
    AppLocalizations strings,
    LibraryOrganizationDecision? decision,
    bool canFindRelated,
  ) => [
    _PhotoEntry(_PhotoAction.open, strings.open, Symbols.open_in_full_rounded),
    if (canFindRelated)
      _PhotoEntry(
        _PhotoAction.related,
        strings.findRelatedPhotos,
        Symbols.image_search_rounded,
      ),
    if (decision != null) ...[
      _PhotoEntry(
        _PhotoAction.favorite,
        decision.favorite ? strings.removeFavorite : strings.addFavorite,
        decision.favorite
            ? Symbols.heart_minus_rounded
            : Symbols.favorite_rounded,
      ),
      _PhotoEntry(
        _PhotoAction.album,
        _albumLabel(strings, decision),
        decision.album == null
            ? Symbols.bookmark_add_rounded
            : Symbols.drive_file_move_rounded,
      ),
      if (decision.album != null)
        _PhotoEntry(
          _PhotoAction.removeAlbum,
          strings.removeFromAlbum,
          Symbols.bookmark_remove_rounded,
        ),
    ],
    _PhotoEntry(
      _PhotoAction.information,
      strings.viewInformation,
      Symbols.info_rounded,
    ),
    _PhotoEntry(
      _PhotoAction.copyFile,
      strings.copyToFolder,
      Symbols.content_copy_rounded,
    ),
    _PhotoEntry(
      _PhotoAction.moveFile,
      strings.moveToFolder,
      Symbols.drive_file_move_rounded,
    ),
    _PhotoEntry(_PhotoAction.renameFile, strings.rename, Symbols.edit_rounded),
    _PhotoEntry(
      _PhotoAction.deleteFile,
      strings.deleteAction,
      Symbols.delete_rounded,
    ),
    _PhotoEntry(
      _PhotoAction.copyPath,
      strings.copyPath,
      Symbols.content_copy_rounded,
    ),
    _PhotoEntry(
      _PhotoAction.reveal,
      strings.openInExplorer,
      Symbols.folder_open_rounded,
    ),
  ];

  String _albumLabel(
    AppLocalizations strings,
    LibraryOrganizationDecision decision,
  ) {
    if (decision.hasConflict) return strings.chooseAlbum;
    if (decision.album != null) return strings.moveToAnotherAlbum;
    return strings.addToAlbum;
  }
}

enum _PhotoAction {
  open,
  related,
  favorite,
  album,
  removeAlbum,
  information,
  copyFile,
  moveFile,
  renameFile,
  deleteFile,
  copyPath,
  reveal;

  bool get changesOrganization => switch (this) {
    favorite || album || removeAlbum => true,
    _ => false,
  };
  bool get changesFiles => switch (this) {
    copyFile || moveFile || renameFile || deleteFile => true,
    _ => false,
  };
}

class _PhotoEntry {
  const _PhotoEntry(this.action, this.label, this.icon);
  final _PhotoAction action;
  final String label;
  final IconData icon;
}
