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
        subtitle: const Text("在本机分析图片，自动生成分类"),
        trailing: TextButton(
          onPressed: () => showLibraryClassificationSettings(context),
          child: const Text("设置"),
        ),
      ),
    ],
  );
}
