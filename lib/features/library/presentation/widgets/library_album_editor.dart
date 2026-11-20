import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_dialog.dart";
import "../../../../app/presentation/ame_disclosure.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_message_text.dart";
import "../../../../app/presentation/ame_overlay_semantics.dart";
import "../../application/library_album_requests.dart";
import "../../application/library_organization_actions.dart";
import "../../domain/library_organization.dart";
import "../library_organization_messages.dart";
import "library_album_icon.dart";

Future<LibraryOrganizationReceipt?> showLibraryAlbumEditor({
  required BuildContext context,
  required LibraryAlbumEditKind kind,
  required BigInt revision,
  LibraryOrganizationAlbum? album,
}) => showDialog<LibraryOrganizationReceipt>(
  context: context,
  builder: (_) =>
      LibraryAlbumEditor(kind: kind, revision: revision, album: album),
);

class LibraryAlbumEditor extends ConsumerStatefulWidget {
  const LibraryAlbumEditor({
    required this.kind,
    required this.revision,
    this.album,
    super.key,
  });
  final LibraryAlbumEditKind kind;
  final BigInt revision;
  final LibraryOrganizationAlbum? album;

  @override
  ConsumerState<LibraryAlbumEditor> createState() => _LibraryAlbumEditorState();
}

class _LibraryAlbumEditorState extends ConsumerState<LibraryAlbumEditor> {
  late final _name = TextEditingController(text: widget.album?.name ?? "");
  late LibraryAlbumIcon _icon = widget.album?.icon ?? LibraryAlbumIcon.album;
  late final _session = LibraryAlbumEditSession(
    kind: widget.kind,
    revision: widget.revision,
    album: widget.album,
  );
  bool _saving = false;
  bool _choosingIcon = false;
  String? _errorCode;

  @override
  void dispose() {
    _session.dispose();
    _name.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (_saving) return;
    final name = _name.text.trim();
    if ((widget.kind == LibraryAlbumEditKind.create ||
            widget.kind == LibraryAlbumEditKind.rename) &&
        (name.isEmpty ||
            name.runes.length > 128 ||
            name.runes.any((rune) => rune < 32))) {
      setState(() => _errorCode = "organization_album_name_invalid");
      return;
    }
    setState(() {
      _saving = true;
      _errorCode = null;
    });
    final result = await ref
        .read(libraryOrganizationActionsProvider.notifier)
        .submit(_session.request(name, icon: _icon));
    if (!mounted) return;
    setState(() => _saving = false);
    switch (result) {
      case LibraryOrganizationSaved(:final receipt):
        Navigator.of(context).pop(receipt);
      case LibraryOrganizationRejected(:final failure):
        setState(() => _errorCode = failure.code);
      case LibraryOrganizationRetired():
        break;
    }
  }

  @override
  Widget build(BuildContext context) {
    final deleting = widget.kind == LibraryAlbumEditKind.delete;
    final changingIcon = widget.kind == LibraryAlbumEditKind.icon;
    final nameError =
        !deleting &&
        !changingIcon &&
        const {
          "organization_album_name_invalid",
          "organization_album_name_exists",
        }.contains(_errorCode);
    final busy = ref.watch(
      libraryOrganizationActionsProvider.select((state) => state.saving),
    );
    final title = switch (widget.kind) {
      LibraryAlbumEditKind.create => context.l10n.createAlbum,
      LibraryAlbumEditKind.rename => context.l10n.renameAlbum,
      LibraryAlbumEditKind.icon => context.l10n.changeAlbumIcon(
        widget.album!.name,
      ),
      LibraryAlbumEditKind.delete => context.l10n.confirmDeleteAlbum,
    };
    var actionLabel = deleting ? context.l10n.deleteAction : context.l10n.save;
    if (_saving) actionLabel = context.l10n.saving;
    return PopScope(
      canPop: !_saving,
      child: AmeDialog(
        key: const Key("library-album-editor"),
        title: title,
        content: AmeDialogBody(
          children: [
            if (changingIcon)
              LibraryAlbumIconPicker(
                value: _icon,
                onChanged: _saving
                    ? null
                    : (icon) => setState(() => _icon = icon),
              ),
            if (deleting) ...[
              AmeDialogItem(
                icon: libraryAlbumIcon(widget.album!.icon),
                title: widget.album!.name,
              ),
              Text(context.l10n.deleteAlbumImpact),
            ] else if (!changingIcon)
              Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  TextField(
                    key: const Key("library-album-name"),
                    controller: _name,
                    autofocus: true,
                    enabled: !_saving,
                    decoration: InputDecoration(
                      labelText: context.l10n.albumName,
                      errorText: nameError
                          ? libraryOrganizationErrorMessage(
                              _errorCode!,
                            ).resolve(context.l10n)
                          : null,
                      icon: widget.kind == LibraryAlbumEditKind.create
                          ? SizedBox.square(
                              dimension: 48,
                              child: AmeTooltip(
                                message: context.l10n.chooseAlbumIcon,
                                child: IconButton.filledTonal(
                                  key: const Key("library-album-choose-icon"),
                                  isSelected: _choosingIcon,
                                  icon: Icon(libraryAlbumIcon(_icon)),
                                  onPressed: _saving
                                      ? null
                                      : () => setState(
                                          () => _choosingIcon = !_choosingIcon,
                                        ),
                                ),
                              ),
                            )
                          : null,
                    ),
                    onSubmitted: (_) => _save(),
                  ),
                  if (widget.kind == LibraryAlbumEditKind.create)
                    AmeDisclosure(
                      child: _choosingIcon
                          ? SizedBox(
                              width: double.infinity,
                              child: Padding(
                                padding: const EdgeInsets.only(top: 16),
                                child: LibraryAlbumIconPicker(
                                  value: _icon,
                                  onChanged: _saving
                                      ? null
                                      : (icon) => setState(() {
                                          _icon = icon;
                                          _choosingIcon = false;
                                        }),
                                ),
                              ),
                            )
                          : null,
                    ),
                ],
              ),
            if (_errorCode case final code? when !nameError)
              AmeDialogError(
                libraryOrganizationErrorMessage(code).resolve(context.l10n),
              ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: _saving ? null : () => Navigator.of(context).pop(),
            child: Text(context.l10n.cancel),
          ),
          FilledButton(
            key: const Key("library-album-save"),
            onPressed: busy ? null : _save,
            child: Text(actionLabel),
          ),
        ],
      ),
    );
  }
}
