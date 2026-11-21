import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/messages/ame_message.dart";
import "../../../../app/presentation/ame_choice.dart";
import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_scroll_motion.dart";
import "../../../../app/presentation/ame_typography.dart";
import "../../../../app/presentation/ame_workbench_surfaces.dart";
import "../../../../app/window/ame_build_identity.dart";
import "../../application/ame_preferences.dart";
import "../../domain/ame_preferences.dart";
import "../widgets/album_settings_section.dart";
import "../widgets/analysis_settings_section.dart";
import "../widgets/organization_archive_section.dart";
import "../widgets/settings_section.dart";
import "../widgets/storage_settings_section.dart";

class AmeSettingsPage extends ConsumerWidget {
  const AmeSettingsPage({
    required this.hasLibraryRoots,
    this.libraryRootIds = const <String>{},
    super.key,
  });

  final bool hasLibraryRoots;
  final Set<String> libraryRootIds;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final preferences = ref.watch(amePreferencesControllerProvider);
    final strings = context.l10n;
    final buildIdentity = ref.watch(ameBuildIdentityProvider);
    return ColoredBox(
      color: AmeWorkbenchSurfaces.of(context).content,
      child: AmeScrollMotion(
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
                              strings.settingsTitle,
                              style: Theme.of(context).textTheme.headlineMedium
                                  ?.copyWith(fontWeight: ameFontWeightRegular),
                            ),
                          ],
                        ),
                        const SizedBox(height: 36),
                        SettingsSection(
                          title: strings.settingsPersonalization,
                          children: [
                            SettingsRow(
                              icon: Symbols.language_rounded,
                              title: strings.settingsLanguage,
                              subtitle: Text(strings.settingsLanguageHelp),
                              trailing: AmeChoice<AmeLanguagePreference>.menu(
                                key: const Key("ame-language-choice"),
                                value: preferences.language,
                                entries: [
                                  AmeChoiceEntry(
                                    value: AmeLanguagePreference.system,
                                    label: strings.languageSystem,
                                  ),
                                  AmeChoiceEntry(
                                    value: AmeLanguagePreference.zhHans,
                                    label: strings.languageChinese,
                                  ),
                                  AmeChoiceEntry(
                                    value: AmeLanguagePreference.en,
                                    label: strings.languageEnglish,
                                  ),
                                  AmeChoiceEntry(
                                    value: AmeLanguagePreference.ja,
                                    label: strings.languageJapanese,
                                  ),
                                ],
                                onSelected: (value) {
                                  if (value != null) {
                                    unawaited(
                                      _savePreferences(
                                        ref,
                                        preferences.copyWith(language: value),
                                      ),
                                    );
                                  }
                                },
                              ),
                            ),
                            SettingsRow(
                              icon: Symbols.palette_rounded,
                              title: strings.settingsTheme,
                              subtitle: Text(strings.settingsThemeHelp),
                              trailing: AmeChoice<AmeThemePreference>.menu(
                                key: const Key("ame-theme-choice"),
                                value: preferences.theme,
                                entries: [
                                  AmeChoiceEntry(
                                    value: AmeThemePreference.system,
                                    label: strings.languageSystem,
                                  ),
                                  AmeChoiceEntry(
                                    value: AmeThemePreference.light,
                                    label: strings.themeLight,
                                  ),
                                  AmeChoiceEntry(
                                    value: AmeThemePreference.dark,
                                    label: strings.themeDark,
                                  ),
                                ],
                                onSelected: (value) {
                                  if (value != null) {
                                    unawaited(
                                      _savePreferences(
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
                          title: strings.settingsBrowsing,
                          children: [
                            SettingsRow(
                              icon: Symbols.mouse_rounded,
                              title: strings.settingsWheel,
                              subtitle: Text(strings.settingsWheelHelp),
                              trailing:
                                  AmeChoice<ImageViewerWheelBehavior>.menu(
                                    value: preferences.viewerWheelBehavior,
                                    entries: [
                                      AmeChoiceEntry(
                                        value: ImageViewerWheelBehavior.zoom,
                                        label: strings.settingsWheelZoom,
                                      ),
                                      AmeChoiceEntry(
                                        value: ImageViewerWheelBehavior
                                            .previousOrNext,
                                        label: strings.settingsWheelNavigate,
                                      ),
                                    ],
                                    onSelected: (value) {
                                      if (value != null) {
                                        unawaited(
                                          _savePreferences(
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
                              title: strings.settingsOpenPhoto,
                              subtitle: Text(strings.settingsOpenPhotoHelp),
                              trailing: AmeChoice<ImageViewerOpenBehavior>.menu(
                                value: preferences.viewerOpenBehavior,
                                entries: [
                                  AmeChoiceEntry(
                                    value: ImageViewerOpenBehavior.automatic,
                                    label: strings.viewerAutomatic,
                                  ),
                                  AmeChoiceEntry(
                                    value: ImageViewerOpenBehavior.fitWindow,
                                    label: strings.viewerFitWindow,
                                  ),
                                  AmeChoiceEntry(
                                    value: ImageViewerOpenBehavior.actualSize,
                                    label: strings.viewerActualSize,
                                  ),
                                ],
                                onSelected: (value) {
                                  if (value != null) {
                                    unawaited(
                                      _savePreferences(
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
                              title: strings.settingsThumbnailSpeed,
                              subtitle: Text(
                                strings.settingsThumbnailSpeedHelp,
                              ),
                              trailing: AmeChoice<PreviewLoadingSpeed>.menu(
                                value: preferences.previewLoadingSpeed,
                                entries: [
                                  AmeChoiceEntry(
                                    value: PreviewLoadingSpeed.small,
                                    label: strings.sizeSmall,
                                  ),
                                  AmeChoiceEntry(
                                    value: PreviewLoadingSpeed.medium,
                                    label: strings.sizeMedium,
                                  ),
                                  AmeChoiceEntry(
                                    value: PreviewLoadingSpeed.large,
                                    label: strings.sizeLarge,
                                  ),
                                ],
                                onSelected: (value) {
                                  if (value != null) {
                                    unawaited(
                                      _savePreferences(
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
                        const AlbumSettingsSection(),
                        const SizedBox(height: 28),
                        const AnalysisSettingsSection(),
                        const SizedBox(height: 28),
                        const OrganizationArchiveSection(),
                        const SizedBox(height: 28),
                        StorageSettingsSection(
                          hasLibraryRoots: hasLibraryRoots,
                          libraryRootIds: libraryRootIds,
                        ),
                        const SizedBox(height: 28),
                        SettingsSection(
                          title: strings.settingsAbout,
                          children: [
                            SettingsRow(
                              icon: Symbols.info_rounded,
                              title: strings.settingsVersion,
                              subtitle: Text(buildIdentity.applicationLabel),
                            ),
                            SettingsRow(
                              key: const Key("open-source-notices-setting"),
                              icon: Symbols.code_rounded,
                              title: strings.settingsLicenses,
                              subtitle: Text(strings.settingsLicensesHelp),
                              trailing: const Icon(
                                Symbols.chevron_right_rounded,
                              ),
                              onTap: () => showLicensePage(
                                context: context,
                                applicationName: "Cedarflake Ame",
                                applicationVersion: buildIdentity.version,
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
      ),
    );
  }

  Future<void> _savePreferences(
    WidgetRef ref,
    AmePreferences preferences,
  ) async {
    try {
      await ref
          .read(amePreferencesControllerProvider.notifier)
          .update(preferences);
    } on Object catch (error) {
      ref
          .read(ameNotificationControllerProvider.notifier)
          .publish(
            AmeNotificationDraft(
              title: AmeLabel.settingsSaveFailed,
              message: AmeLabel.operationFailedDetails,
              detail: AmeRawMessage(error.toString()),
              severity: AmeNotificationSeverity.error,
              isPersistent: true,
            ),
          );
    }
  }
}
