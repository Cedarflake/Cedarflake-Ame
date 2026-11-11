import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_dialog.dart";
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
      LibraryAlbumEditKind.create => "新建相册",
      LibraryAlbumEditKind.rename => "重命名相册",
      LibraryAlbumEditKind.icon => "更改 ${widget.album!.name} 的图标",
      LibraryAlbumEditKind.delete => "删除相册？",
    };
    var actionLabel = deleting ? "删除" : "保存";
    if (_saving) actionLabel = "正在保存";
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
              const Text("其中的图片组将进入已忽略。图片文件和收藏状态会保留。"),
            ] else if (!changingIcon)
              TextField(
                key: const Key("library-album-name"),
                controller: _name,
                autofocus: true,
                enabled: !_saving,
                decoration: InputDecoration(
                  labelText: "相册名称",
                  errorText: nameError
                      ? libraryOrganizationErrorMessage(_errorCode!)
                      : null,
                  icon: widget.kind == LibraryAlbumEditKind.create
                      ? SizedBox.square(
                          dimension: 48,
                          child: IconButton.filledTonal(
                            key: const Key("library-album-choose-icon"),
                            tooltip: "选择相册图标",
                            isSelected: _choosingIcon,
                            icon: Icon(libraryAlbumIcon(_icon)),
                            onPressed: _saving
                                ? null
                                : () => setState(
                                    () => _choosingIcon = !_choosingIcon,
                                  ),
                          ),
                        )
                      : null,
                ),
                onSubmitted: (_) => _save(),
              ),
            if (widget.kind == LibraryAlbumEditKind.create &&
                _choosingIcon) ...[
              LibraryAlbumIconPicker(
                value: _icon,
                onChanged: _saving
                    ? null
                    : (icon) => setState(() {
                        _icon = icon;
                        _choosingIcon = false;
                      }),
              ),
            ],
            if (_errorCode case final code? when !nameError)
              AmeDialogError(libraryOrganizationErrorMessage(code)),
          ],
        ),
        actions: [
          TextButton(
            onPressed: _saving ? null : () => Navigator.of(context).pop(),
            child: const Text("取消"),
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
