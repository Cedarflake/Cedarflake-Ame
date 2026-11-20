import "package:material_ui/material_ui.dart";
import "package:flutter/services.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../features/library/application/ports/library_catalog.dart";
import "../../features/settings/domain/ame_preferences.dart";
import "../../features/settings/presentation/ame_language_preference.dart";
import "../../features/settings/presentation/ame_theme_preference.dart";
import "../presentation/ame_localizations.dart";
import "../presentation/ame_system_theme.dart";
import "../presentation/ame_theme.dart";
import "../presentation/ame_transient_feedback.dart";
import "../window/ame_build_identity.dart";
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
        locale: amePreferredLocale(preferences.language),
        localeListResolutionCallback: resolveAmeLocales,
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
                          context.l10n.startupFailureTitle,
                          style: Theme.of(context).textTheme.headlineSmall,
                          textAlign: TextAlign.center,
                        ),
                        const SizedBox(height: 12),
                        Text(
                          bootstrapFailureMessage(context.l10n, error),
                          textAlign: TextAlign.center,
                        ),
                        const SizedBox(height: 20),
                        OutlinedButton.icon(
                          onPressed: () => _copyDiagnostics(context),
                          icon: const Icon(Symbols.content_copy_rounded),
                          label: Text(context.l10n.copyDiagnostics),
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
    await Clipboard.setData(
      ClipboardData(
        text: const AmeBuildIdentity.current().startupDiagnostics(error),
      ),
    );
    if (!context.mounted) {
      return;
    }
    showAmeTransientFeedback(
      context,
      message: context.l10n.diagnosticsCopied,
      severity: AmeNotificationSeverity.success,
    );
  }
}

String bootstrapFailureMessage(AppLocalizations strings, Object error) {
  if (error case LibraryCatalogFailure(:final code)) {
    return switch (code) {
      "catalog_schema_unsupported" => strings.startupUnsupportedCatalog,
      "catalog_database_error" => strings.startupDatabaseError,
      _ => strings.startupCatalogError,
    };
  }
  return strings.startupComponentError;
}
