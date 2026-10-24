import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../application/library_organization_conflicts.dart";
import "../../domain/library_organization.dart";

class LibraryConflictAlbumsView extends ConsumerStatefulWidget {
  const LibraryConflictAlbumsView({required this.target, super.key});
  final LibraryOrganizationTarget target;
  @override
  ConsumerState<LibraryConflictAlbumsView> createState() =>
      _LibraryConflictAlbumsViewState();
}

class _LibraryConflictAlbumsViewState
    extends ConsumerState<LibraryConflictAlbumsView> {
  String? _after;
  final List<String?> _previous = [];

  @override
  void didUpdateWidget(LibraryConflictAlbumsView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.target != widget.target) {
      _after = null;
      _previous.clear();
    }
  }

  @override
  Widget build(BuildContext context) {
    final request = (target: widget.target, after: _after);
    final page = ref.watch(libraryConflictAlbumsProvider(request));
    return Padding(
      padding: const EdgeInsets.only(bottom: 16),
      child: page.when(
        skipLoadingOnRefresh: false,
        loading: () => const LinearProgressIndicator(),
        error: (_, _) => Row(
          children: [
            const Expanded(child: Text("无法读取冲突涉及的相册。")),
            TextButton(
              onPressed: () =>
                  ref.invalidate(libraryConflictAlbumsProvider(request)),
              child: const Text("重试"),
            ),
          ],
        ),
        data: (value) => Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(value.albums.isEmpty ? "相册归属已更新。" : "冲突涉及的相册"),
            if (value.albums.isNotEmpty)
              ConstrainedBox(
                constraints: const BoxConstraints(maxHeight: 128),
                child: AmeScrollMotion(
                  child: SingleChildScrollView(
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        for (final album in value.albums)
                          Padding(
                            padding: const EdgeInsets.symmetric(vertical: 4),
                            child: Text(
                              album.retired ? "${album.name}（已删除）" : album.name,
                            ),
                          ),
                      ],
                    ),
                  ),
                ),
              ),
            if (_previous.isNotEmpty || value.nextAlbumId != null)
              Row(
                children: [
                  TextButton(
                    onPressed: _previous.isEmpty
                        ? null
                        : () => setState(() => _after = _previous.removeLast()),
                    child: const Text("上一页"),
                  ),
                  const Spacer(),
                  TextButton(
                    onPressed: value.nextAlbumId == null
                        ? null
                        : () => setState(() {
                            _previous.add(_after);
                            _after = value.nextAlbumId;
                          }),
                    child: const Text("下一页"),
                  ),
                ],
              ),
          ],
        ),
      ),
    );
  }
}
