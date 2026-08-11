import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../application/ame_preferences.dart";
import "widgets/settings_section.dart";
import "widgets/storage_settings_section.dart";

class AmeSettingsPage extends ConsumerWidget {
  const AmeSettingsPage({required this.hasLibraryRoots, super.key});

  final bool hasLibraryRoots;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final preferences = ref.watch(amePreferencesControllerProvider);
    return ColoredBox(
      color: Theme.of(context).colorScheme.surfaceContainerLowest,
      child: CustomScrollView(
        key: const Key("ame-settings-page"),
        slivers: [
          SliverPadding(
            padding: const EdgeInsets.fromLTRB(32, 24, 32, 64),
            sliver: SliverToBoxAdapter(
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 980),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: [
                      Row(
                        children: [
                          const Icon(Symbols.settings_rounded, size: 30),
                          const SizedBox(width: 12),
                          Text(
                            "设置",
                            style: Theme.of(context).textTheme.headlineMedium,
                          ),
                        ],
                      ),
                      const SizedBox(height: 36),
                      SettingsSection(
                        title: "个性化",
                        children: [
                          SettingsRow(
                            icon: Symbols.palette_rounded,
                            title: "应用主题",
                            subtitle: const Text("选择 Ame 使用的明暗外观"),
                            trailing: SettingsChoice<AmeThemePreference>(
                              value: preferences.theme,
                              entries: const [
                                DropdownMenuEntry(
                                  value: AmeThemePreference.system,
                                  label: "跟随系统",
                                ),
                                DropdownMenuEntry(
                                  value: AmeThemePreference.light,
                                  label: "浅色",
                                ),
                                DropdownMenuEntry(
                                  value: AmeThemePreference.dark,
                                  label: "深色",
                                ),
                              ],
                              onSelected: (value) {
                                if (value != null) {
                                  unawaited(
                                    _savePreferences(
                                      context,
                                      ref,
                                      preferences.copyWith(theme: value),
                                    ),
                                  );
                                }
                              },
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 28),
                      SettingsSection(
                        title: "浏览",
                        children: [
                          SettingsRow(
                            icon: Symbols.mouse_rounded,
                            title: "鼠标滚轮",
                            subtitle: const Text("查看单张图片时滚动滚轮的行为"),
                            trailing: SettingsChoice<ImageViewerWheelBehavior>(
                              value: preferences.viewerWheelBehavior,
                              entries: const [
                                DropdownMenuEntry(
                                  value: ImageViewerWheelBehavior.zoom,
                                  label: "放大或缩小",
                                ),
                                DropdownMenuEntry(
                                  value:
                                      ImageViewerWheelBehavior.previousOrNext,
                                  label: "上一张或下一张",
                                ),
                              ],
                              onSelected: (value) {
                                if (value != null) {
                                  unawaited(
                                    _savePreferences(
                                      context,
                                      ref,
                                      preferences.copyWith(
                                        viewerWheelBehavior: value,
                                      ),
                                    ),
                                  );
                                }
                              },
                            ),
                          ),
                          SettingsRow(
                            icon: Symbols.image_rounded,
                            title: "打开图片",
                            subtitle: const Text("选择图片首次打开时的显示大小"),
                            trailing: SettingsChoice<ImageViewerOpenBehavior>(
                              value: preferences.viewerOpenBehavior,
                              entries: const [
                                DropdownMenuEntry(
                                  value: ImageViewerOpenBehavior.fitWindow,
                                  label: "适应窗口",
                                ),
                                DropdownMenuEntry(
                                  value: ImageViewerOpenBehavior.actualSize,
                                  label: "实际大小",
                                ),
                              ],
                              onSelected: (value) {
                                if (value != null) {
                                  unawaited(
                                    _savePreferences(
                                      context,
                                      ref,
                                      preferences.copyWith(
                                        viewerOpenBehavior: value,
                                      ),
                                    ),
                                  );
                                }
                              },
                            ),
                          ),
                          SettingsRow(
                            key: const Key("preview-loading-speed-setting"),
                            icon: Symbols.speed_rounded,
                            title: "缩略图加载速度",
                            subtitle: const Text("档位越大，同时加载越多，占用资源也越高"),
                            trailing: SettingsChoice<PreviewLoadingSpeed>(
                              value: preferences.previewLoadingSpeed,
                              entries: const [
                                DropdownMenuEntry(
                                  value: PreviewLoadingSpeed.small,
                                  label: "小",
                                ),
                                DropdownMenuEntry(
                                  value: PreviewLoadingSpeed.medium,
                                  label: "中",
                                ),
                                DropdownMenuEntry(
                                  value: PreviewLoadingSpeed.large,
                                  label: "大",
                                ),
                              ],
                              onSelected: (value) {
                                if (value != null) {
                                  unawaited(
                                    _savePreferences(
                                      context,
                                      ref,
                                      preferences.copyWith(
                                        previewLoadingSpeed: value,
                                      ),
                                    ),
                                  );
                                }
                              },
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 28),
                      StorageSettingsSection(hasLibraryRoots: hasLibraryRoots),
                      const SizedBox(height: 28),
                      SettingsSection(
                        title: "关于",
                        children: [
                          const SettingsRow(
                            icon: Symbols.info_rounded,
                            title: "版本",
                            subtitle: Text("Cedarflake Ame 0.1.0"),
                          ),
                          SettingsRow(
                            key: const Key("open-source-notices-setting"),
                            icon: Symbols.code_rounded,
                            title: "开源软件声明",
                            subtitle: const Text("查看 Ame 使用的开源软件与许可"),
                            trailing: const Icon(Symbols.chevron_right_rounded),
                            onTap: () => showLicensePage(
                              context: context,
                              applicationName: "Cedarflake Ame",
                              applicationVersion: "0.1.0",
                            ),
                          ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  Future<void> _savePreferences(
    BuildContext context,
    WidgetRef ref,
    AmePreferences preferences,
  ) async {
    try {
      await ref
          .read(amePreferencesControllerProvider.notifier)
          .update(preferences);
    } on Object catch (error) {
      if (context.mounted) {
        ScaffoldMessenger.of(context)
          ..hideCurrentSnackBar()
          ..showSnackBar(SnackBar(content: Text("无法保存设置：$error")));
      }
    }
  }
}
