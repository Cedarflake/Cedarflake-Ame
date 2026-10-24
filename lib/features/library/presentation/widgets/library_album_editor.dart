import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_album_requests.dart";
import "../../application/library_organization_actions.dart";
import "../../domain/library_organization.dart";
import "../library_organization_messages.dart";

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
  late final _session = LibraryAlbumEditSession(
    kind: widget.kind,
    revision: widget.revision,
    album: widget.album,
  );
  bool _saving = false;
  String? _error;

  @override
  void dispose() {
    _session.dispose();
    _name.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    if (_saving) return;
    final name = _name.text.trim();
    if (widget.kind != LibraryAlbumEditKind.delete &&
        (name.isEmpty ||
            name.runes.length > 128 ||
            name.runes.any((rune) => rune < 32))) {
      setState(
        () => _error = libraryOrganizationErrorMessage(
          "organization_album_name_invalid",
        ),
      );
      return;
    }
    setState(() {
      _saving = true;
      _error = null;
    });
    final result = await ref
        .read(libraryOrganizationActionsProvider.notifier)
        .submit(_session.request(name));
    if (!mounted) return;
    setState(() => _saving = false);
    switch (result) {
      case LibraryOrganizationSaved(:final receipt):
        Navigator.of(context).pop(receipt);
      case LibraryOrganizationRejected(:final failure):
        setState(() => _error = libraryOrganizationErrorMessage(failure.code));
      case LibraryOrganizationRetired():
        break;
    }
  }

  @override
  Widget build(BuildContext context) {
    final deleting = widget.kind == LibraryAlbumEditKind.delete;
    final busy = ref.watch(
      libraryOrganizationActionsProvider.select((state) => state.saving),
    );
    final title = switch (widget.kind) {
      LibraryAlbumEditKind.create => "新建相册",
      LibraryAlbumEditKind.rename => "重命名相册",
      LibraryAlbumEditKind.delete => "删除相册？",
    };
    var actionLabel = deleting ? "删除" : "保存";
    if (_saving) actionLabel = "正在保存";
    return PopScope(
      canPop: !_saving,
      child: AlertDialog(
        key: const Key("library-album-editor"),
        title: Text(title),
        content: SizedBox(
          width: 400,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (deleting)
                Text("删除“${widget.album!.name}”后，其中的图片组将进入已忽略。图片文件和收藏状态会保留。")
              else
                TextField(
                  key: const Key("library-album-name"),
                  controller: _name,
                  autofocus: true,
                  enabled: !_saving,
                  decoration: InputDecoration(
                    labelText: "相册名称",
                    errorText: _error,
                  ),
                  onSubmitted: (_) => _save(),
                ),
              if (deleting && _error != null)
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: Text(
                    _error!,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ),
            ],
          ),
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
