import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../library/application/library_album_pages.dart";
import "../../application/ame_preferences.dart";
import "../../domain/album_preferences.dart";
import "../default_album_picker.dart";
import "settings_section.dart";

class AlbumSettingsSection extends ConsumerStatefulWidget {
  const AlbumSettingsSection({super.key});

  @override
  ConsumerState<AlbumSettingsSection> createState() =>
      _AlbumSettingsSectionState();
}

class _AlbumSettingsSectionState extends ConsumerState<AlbumSettingsSection> {
  bool _saving = false;
  String? _error;

  Future<void> _save(AlbumPreferences albums) async {
    setState(() {
      _saving = true;
      _error = null;
    });
    try {
      final preferences = ref.read(amePreferencesControllerProvider);
      await ref
          .read(amePreferencesControllerProvider.notifier)
          .update(preferences.copyWith(albums: albums));
    } on Object {
      if (mounted) setState(() => _error = "设置未能保存，请重试。");
    } finally {
      if (mounted) setState(() => _saving = false);
    }
  }

  Future<void> _choose() async {
    final id = ref.read(amePreferencesControllerProvider).albums.defaultAlbumId;
    final result = await ref.read(defaultAlbumPickerProvider)(context, id);
    if (!mounted || result == null) return;
    await _save(
      AlbumPreferences(
        askBeforeAdding: ref
            .read(amePreferencesControllerProvider)
            .albums
            .askBeforeAdding,
        defaultAlbumId: result.albumId,
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final albums = ref.watch(amePreferencesControllerProvider).albums;
    final id = albums.defaultAlbumId;
    final selected = id == null ? null : ref.watch(libraryAlbumProvider(id));
    final label =
        selected?.when(
          data: (album) => album?.name ?? "未设置",
          error: (error, stack) => "无法读取",
          loading: () => "正在加载",
        ) ??
        "未设置";
    return SettingsSection(
      title: "相册",
      children: [
        SettingsRow(
          icon: Symbols.photo_album_rounded,
          title: "加入相册前询问",
          subtitle: const Text("每次加入前选择目标相册"),
          trailing: Switch(
            key: const Key("settings-album-ask"),
            value: albums.askBeforeAdding,
            onChanged: _saving
                ? null
                : (value) => _save(
                    AlbumPreferences(
                      askBeforeAdding: value,
                      defaultAlbumId: albums.defaultAlbumId,
                    ),
                  ),
          ),
        ),
        SettingsRow(
          icon: Symbols.bookmark_rounded,
          title: "默认加入的相册",
          subtitle: const Text("未设置时，会在加入前询问"),
          trailing: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 176),
            child: TextButton.icon(
              key: const Key("settings-default-album"),
              onPressed: _saving ? null : _choose,
              icon: const Icon(Symbols.chevron_right_rounded),
              iconAlignment: IconAlignment.end,
              label: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
            ),
          ),
        ),
        if (_error case final error?)
          Padding(
            padding: const EdgeInsets.all(20),
            child: Text(
              error,
              style: TextStyle(color: Theme.of(context).colorScheme.error),
            ),
          ),
      ],
    );
  }
}
