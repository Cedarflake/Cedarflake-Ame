import "dart:async";

import "package:flutter/material.dart";

import "../../domain/library_organization_batch.dart";

class LibrarySelectionProgress extends StatefulWidget {
  const LibrarySelectionProgress({this.progress, super.key});

  final LibraryBatchProgress? progress;

  @override
  State<LibrarySelectionProgress> createState() =>
      _LibrarySelectionProgressState();
}

class _LibrarySelectionProgressState extends State<LibrarySelectionProgress> {
  late final Timer _delay;
  bool _visible = false;

  @override
  void initState() {
    super.initState();
    _delay = Timer(const Duration(milliseconds: 500), () {
      if (mounted) {
        setState(() => _visible = true);
      }
    });
  }

  @override
  void dispose() {
    _delay.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (!_visible) return const SizedBox.shrink();
    final progress = widget.progress;
    var label = progress?.phase == LibraryBatchPhase.saving ? "正在保存" : "正在准备";
    if (progress != null && progress.total > 0) {
      label += "（${progress.completed}/${progress.total}）";
    }
    return LinearProgressIndicator(
      key: const Key("library-selection-progress"),
      minHeight: 2,
      semanticsLabel: label,
    );
  }
}
