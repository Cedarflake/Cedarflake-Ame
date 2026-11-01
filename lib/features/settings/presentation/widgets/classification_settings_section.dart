import "package:flutter/material.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../library/presentation/widgets/library_classification_settings_editor.dart";
import "settings_section.dart";

class ClassificationSettingsSection extends StatelessWidget {
  const ClassificationSettingsSection({super.key});

  @override
  Widget build(BuildContext context) => SettingsSection(
    title: "智能相册",
    children: [
      SettingsRow(
        icon: Symbols.auto_awesome_rounded,
        title: "本地分析",
        subtitle: const Text("启用智能相册，管理分类设置"),
        trailing: TextButton(
          onPressed: () => showLibraryClassificationSettings(context),
          child: const Text("设置"),
        ),
      ),
    ],
  );
}
