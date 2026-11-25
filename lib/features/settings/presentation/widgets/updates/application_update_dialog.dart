import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_markdown_plus/flutter_markdown_plus.dart";

import "../../../../../app/presentation/ame_dialog.dart";
import "../../../../../app/presentation/ame_localizations.dart";
import "../../../../updates/application/application_update_controller.dart";
import "../../../../updates/application/application_update_state.dart";
import "update_text.dart";

Future<void> showApplicationUpdateDialog(
  BuildContext context,
  ApplicationUpdateController controller,
) => showDialog<void>(
  context: context,
  builder: (context) => ApplicationUpdateDialog(controller: controller),
);

class ApplicationUpdateDialog extends StatelessWidget {
  const ApplicationUpdateDialog({required this.controller, super.key});
  final ApplicationUpdateController controller;

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: controller,
    builder: (context, _) {
      final strings = context.l10n;
      final state = controller.state;
      final phase = state.phase;
      final release = state.release;
      final transferring =
          phase == ApplicationUpdatePhase.downloading ||
          phase == ApplicationUpdatePhase.verifying ||
          phase == ApplicationUpdatePhase.stopping;
      final checking =
          phase == ApplicationUpdatePhase.initializing ||
          phase == ApplicationUpdatePhase.checking;
      final ready = phase == ApplicationUpdatePhase.ready;
      final progress = state.progress;
      return AmeDialog(
        title: strings.updateTitle,
        isLoading: checking,
        loadingLabel: strings.updateChecking,
        content: AmeDialogBody(
          children: [
            if (release != null) ...[
              Text(
                "${release.version} · ${updateSize(strings, release.downloadBytes)}",
                style: Theme.of(context).textTheme.titleMedium,
              ),
              if (release.notes.isNotEmpty)
                ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: 240),
                  child: SingleChildScrollView(
                    child: MarkdownBody(
                      data: release.notes,
                      selectable: true,
                      imageBuilder: (_, _, _) => const SizedBox.shrink(),
                      styleSheet: MarkdownStyleSheet(
                        p: Theme.of(context).textTheme.bodyMedium,
                        h1: Theme.of(context).textTheme.titleLarge,
                        h2: Theme.of(context).textTheme.titleMedium,
                        h3: Theme.of(context).textTheme.titleSmall,
                      ),
                    ),
                  ),
                ),
            ],
            if (state.failure case final failure?)
              AmeDialogError(updateFailureLabel(strings, failure))
            else if (!state.supported && !checking)
              Text(strings.updateUnavailable)
            else if (phase == ApplicationUpdatePhase.current)
              Text(strings.updateCurrent)
            else if (ready)
              Text(strings.updateReadyHelp),
            if (transferring) ...[
              LinearProgressIndicator(
                value: progress == null || progress.totalBytes == 0
                    ? 0
                    : (progress.receivedBytes / progress.totalBytes).clamp(
                        0,
                        1,
                      ),
              ),
              Text(switch (phase) {
                ApplicationUpdatePhase.verifying => strings.updateVerifying,
                ApplicationUpdatePhase.stopping => strings.updateStopping,
                _ => strings.updateDownloading,
              }),
            ],
            if (phase == ApplicationUpdatePhase.restarting)
              Text(strings.updateRestarting),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(release == null ? strings.close : strings.updateLater),
          ),
          if (transferring)
            TextButton(
              onPressed: phase == ApplicationUpdatePhase.stopping
                  ? null
                  : controller.cancelDownload,
              child: Text(strings.updateStop),
            )
          else if (release != null)
            FilledButton(
              onPressed: state.busy
                  ? null
                  : () => unawaited(
                      ready ? controller.restart() : controller.download(),
                    ),
              child: Text(
                ready ? strings.updateRestart : strings.updateDownload,
              ),
            )
          else if (state.failure != null)
            FilledButton(
              onPressed: () => unawaited(controller.check()),
              child: Text(strings.retry),
            ),
        ],
      );
    },
  );
}
