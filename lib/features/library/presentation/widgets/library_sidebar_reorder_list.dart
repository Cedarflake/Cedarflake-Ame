import "dart:async";

import "package:flutter/foundation.dart";
import "package:flutter/material.dart";

import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../../../app/presentation/ame_transient_feedback.dart";
import "../../application/library_sidebar_order.dart";
import "../../application/library_sidebar_ordering.dart";

class LibrarySidebarReorderList<T extends Object> extends StatefulWidget {
  const LibrarySidebarReorderList({
    required this.items,
    required this.itemId,
    required this.itemBuilder,
    required this.header,
    this.footer,
    this.onReorder,
    this.onSaved,
    this.enabled = true,
    super.key,
  });

  final List<T> items;
  final String Function(T item) itemId;
  final Widget Function(BuildContext context, T item, int index, bool draggable)
  itemBuilder;
  final Widget Function(bool saving) header;
  final Widget Function(bool saving)? footer;
  final Future<void> Function(LibrarySidebarMove move)? onReorder;
  final VoidCallback? onSaved;
  final bool enabled;

  @override
  State<LibrarySidebarReorderList<T>> createState() =>
      _LibrarySidebarReorderListState<T>();
}

class _LibrarySidebarReorderListState<T extends Object>
    extends State<LibrarySidebarReorderList<T>> {
  int _itemsRevision = 0;
  late final _ordering = LibrarySidebarOrdering(
    ids: widget.items.map(widget.itemId).toList(),
    save: (move) => widget.onReorder!(move),
  );

  @override
  void didUpdateWidget(covariant LibrarySidebarReorderList<T> oldWidget) {
    super.didUpdateWidget(oldWidget);
    final ids = widget.items.map(widget.itemId).toList();
    if (!listEquals(oldWidget.items.map(oldWidget.itemId).toList(), ids)) {
      // Retire framework drag indexes when their source identities change.
      // AmeScrollMotion retains the scroll controller across this replacement.
      _itemsRevision++;
    }
    _ordering.updateItems(ids);
  }

  @override
  void dispose() {
    _ordering.dispose();
    super.dispose();
  }

  Future<void> _reorder(int from, int to) async {
    try {
      final saved = await _ordering.move(from, to);
      if (mounted && saved) widget.onSaved?.call();
    } catch (_) {
      if (!mounted) return;
      showAmeTransientFeedback(
        context,
        message: "排列顺序未能保存，请重新拖动。",
        severity: AmeNotificationSeverity.error,
      );
    }
  }

  @override
  Widget build(BuildContext context) => AnimatedBuilder(
    animation: _ordering,
    builder: (context, _) {
      final items = {
        for (final item in widget.items) widget.itemId(item): item,
      };
      final ids = _ordering.ids;
      final draggable =
          widget.enabled && widget.onReorder != null && !_ordering.saving;
      return AmeScrollMotion(
        child: ReorderableListView.builder(
          key: ValueKey(_itemsRevision),
          padding: const EdgeInsets.fromLTRB(8, 12, 8, 12),
          buildDefaultDragHandles: false,
          itemCount: ids.length,
          onReorderItem: (from, to) {
            if (draggable) unawaited(_reorder(from, to));
          },
          header: widget.header(_ordering.saving),
          footer: widget.footer?.call(_ordering.saving),
          itemBuilder: (context, index) => KeyedSubtree(
            key: ValueKey(ids[index]),
            child: widget.itemBuilder(
              context,
              items[ids[index]]!,
              index,
              draggable,
            ),
          ),
        ),
      );
    },
  );
}
