import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_classification_configuration.dart";
import "../../application/library_model_installation.dart";
import "../../application/library_model_activation.dart";

Future<void> showLibraryModelSetup(BuildContext context, WidgetRef ref) async {
  final configuration = ref.read(libraryClassificationConfigurationProvider);
  final installation = ref.read(libraryModelInstallationProvider);
  if (configuration == null || installation == null || !installation.canStart) {
    return;
  }

  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      key: const Key("classification-model-download"),
      title: const Text("下载智能相册模型"),
      content: const SizedBox(
        width: 480,
        child: Text("需要下载约 504 MB 的模型，之后在本机分析图片。已有的完整模型文件会复用。"),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: const Text("取消"),
        ),
        FilledButton(
          onPressed: () => Navigator.of(context).pop(true),
          child: const Text("下载并启用"),
        ),
      ],
    ),
  );
  if (confirmed != true || !context.mounted || !installation.canStart) return;

  unawaited(installation.start(activateLibraryModel(configuration)));
}
