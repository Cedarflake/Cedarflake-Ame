import "package:material_ui/material_ui.dart";

import "../../../../app/presentation/ame_progress.dart";
import "../../../../app/presentation/ame_feedback_card.dart";
import "../../../../app/notifications/ame_notification_controller.dart";
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
    return AmeProgressVisibility(
      key: const Key("library-selection-progress"),
      delay: const Duration(milliseconds: 500),
      child: AmeFeedbackCard(
        title: label,
        severity: AmeNotificationSeverity.info,
        actions: const [],
        progress: AmeLinearProgress(
          delay: Duration.zero,
          semanticsLabel: label,
        ),
      ),
    );
  }
}
