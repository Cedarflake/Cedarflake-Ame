import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_progress.dart";
import "../../domain/library_organization_batch.dart";

class LibrarySelectionProgress extends StatelessWidget {
  const LibrarySelectionProgress({this.progress, super.key});

  final LibraryBatchProgress? progress;

  @override
  Widget build(BuildContext context) {
    final progress = this.progress;
    var label = progress?.phase == LibraryBatchPhase.saving ? "正在保存" : "正在准备";
    if (progress != null && progress.total > 0) {
      label += "（${progress.completed}/${progress.total}）";
    }
    return AmeLinearProgress(
      key: const Key("library-selection-progress"),
      delay: const Duration(milliseconds: 500),
      minHeight: 2,
      semanticsLabel: label,
    );
  }
}
