import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../features/library/application/library_catalog.dart";
import "../../features/settings/application/ame_preferences.dart";
import "../presentation/ame_localizations.dart";
import "../presentation/ame_system_theme.dart";
import "../presentation/ame_theme.dart";
import "../window/ame_window_frame.dart";

class AmeBootstrapFailure extends StatelessWidget {
  const AmeBootstrapFailure({
    required this.error,
    required this.preferences,
    super.key,
  });

  final Object error;
  final AmePreferences preferences;

  @override
  Widget build(BuildContext context) {
    return AmeSystemThemeBuilder(
      builder: (context, seedColor) => MaterialApp(
        debugShowCheckedModeBanner: false,
        locale: ameLocale,
        supportedLocales: ameSupportedLocales,
        localizationsDelegates: ameLocalizationsDelegates,
        theme: buildAmeTheme(seedColor: seedColor),
        darkTheme: buildAmeTheme(
          brightness: Brightness.dark,
          seedColor: seedColor,
        ),
        themeMode: ameThemeMode(preferences.theme),
        home: Builder(
          builder: (context) => AmeWindowFrame(
            child: Scaffold(
              body: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 560),
                  child: Padding(
                    padding: const EdgeInsets.all(32),
                    child: Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        const Icon(Symbols.error_rounded, size: 48),
                        const SizedBox(height: 20),
                        Text(
                          "Cedarflake Ame 无法启动",
                          style: Theme.of(context).textTheme.headlineSmall,
                          textAlign: TextAlign.center,
                        ),
                        const SizedBox(height: 12),
                        Text(
                          bootstrapFailureMessage(error),
                          textAlign: TextAlign.center,
                        ),
                        const SizedBox(height: 20),
                        OutlinedButton.icon(
                          onPressed: () => _copyDiagnostics(context),
                          icon: const Icon(Symbols.content_copy_rounded),
                          label: const Text("复制诊断信息"),
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Future<void> _copyDiagnostics(BuildContext context) async {
    await Clipboard.setData(ClipboardData(text: error.toString()));
    if (!context.mounted) {
      return;
    }
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(const SnackBar(content: Text("诊断信息已复制")));
  }
}

String bootstrapFailureMessage(Object error) {
  if (error case LibraryCatalogFailure(:final code)) {
    return switch (code) {
      "catalog_schema_unsupported" => "图库数据来自不受支持的版本，无法安全打开。请复制诊断信息以便排查。",
      "catalog_database_error" => "图库数据暂时无法读取。请确认磁盘可用后重新启动；如果仍然失败，请复制诊断信息以便排查。",
      _ => "图库加载失败。请重新启动 Ame；如果仍然失败，请复制诊断信息以便排查。",
    };
  }
  return "应用组件加载失败。请重新启动 Ame；如果仍然失败，请复制诊断信息以便排查。";
}
