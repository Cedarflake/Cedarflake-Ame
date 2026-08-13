import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/presentation/ame_localizations.dart";
import "package:cedarflake_ame/app/presentation/ame_system_theme.dart";
import "package:cedarflake_ame/features/library/presentation/unified_library_screen.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets("uses Simplified Chinese locale and dense text geometry", (
    tester,
  ) async {
    await tester.pumpWidget(const ProviderScope(child: AmeApp()));

    final context = tester.element(find.byType(UnifiedLibraryScreen));
    expect(Localizations.localeOf(context), ameLocale);
    expect(
      MaterialLocalizations.of(context).scriptCategory,
      ScriptCategory.dense,
    );
  });

  testWidgets("derives the Material theme from the system accent color", (
    tester,
  ) async {
    const accent = Color(0xFFB146C2);
    const methodChannel = MethodChannel("cedarflake_ame/system_theme");
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      methodChannel,
      (call) async => _packedColor(accent),
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        methodChannel,
        null,
      ),
    );

    await tester.pumpWidget(const ProviderScope(child: AmeApp()));
    await tester.pumpAndSettle();

    final context = tester.element(find.byType(UnifiedLibraryScreen));
    final expected = ColorScheme.fromSeed(
      seedColor: accent,
      dynamicSchemeVariant: DynamicSchemeVariant.fidelity,
    );
    expect(Theme.of(context).colorScheme.primary, expected.primary);
  });

  testWidgets("falls back when the system accent is unavailable", (
    tester,
  ) async {
    const methodChannel = MethodChannel("cedarflake_ame/system_theme");
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      methodChannel,
      (call) async => throw PlatformException(code: "unavailable"),
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        methodChannel,
        null,
      ),
    );

    await tester.pumpWidget(const ProviderScope(child: AmeApp()));
    await tester.pumpAndSettle();

    final context = tester.element(find.byType(UnifiedLibraryScreen));
    final expected = ColorScheme.fromSeed(
      seedColor: ameFallbackSeedColor,
      dynamicSchemeVariant: DynamicSchemeVariant.fidelity,
    );
    expect(Theme.of(context).colorScheme.primary, expected.primary);
  });

  testWidgets("updates the Material theme when the system accent changes", (
    tester,
  ) async {
    const methodChannel = MethodChannel("cedarflake_ame/system_theme");
    await tester.pumpWidget(const ProviderScope(child: AmeApp()));
    await tester.pump();
    const accent = Color(0xFF2B8F4E);
    unawaited(
      tester.binding.defaultBinaryMessenger.handlePlatformMessage(
        methodChannel.name,
        methodChannel.codec.encodeMethodCall(
          MethodCall("accentColorChanged", _packedColor(accent)),
        ),
        null,
      ),
    );
    await tester.pumpAndSettle();

    final context = tester.element(find.byType(UnifiedLibraryScreen));
    final expected = ColorScheme.fromSeed(
      seedColor: accent,
      dynamicSchemeVariant: DynamicSchemeVariant.fidelity,
    );
    expect(Theme.of(context).colorScheme.primary, expected.primary);
  });

  testWidgets("keeps a newer accent when the startup read finishes late", (
    tester,
  ) async {
    final startupColor = Completer<Object?>();
    const methodChannel = MethodChannel("cedarflake_ame/system_theme");
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      methodChannel,
      (call) => startupColor.future,
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        methodChannel,
        null,
      ),
    );

    await tester.pumpWidget(const ProviderScope(child: AmeApp()));
    const newestAccent = Color(0xFF7D5128);
    unawaited(
      tester.binding.defaultBinaryMessenger.handlePlatformMessage(
        methodChannel.name,
        methodChannel.codec.encodeMethodCall(
          MethodCall("accentColorChanged", _packedColor(newestAccent)),
        ),
        null,
      ),
    );
    await tester.pump();
    startupColor.complete(_packedColor(const Color(0xFF1265C4)));
    await tester.pumpAndSettle();

    final context = tester.element(find.byType(UnifiedLibraryScreen));
    final expected = ColorScheme.fromSeed(
      seedColor: newestAccent,
      dynamicSchemeVariant: DynamicSchemeVariant.fidelity,
    );
    expect(Theme.of(context).colorScheme.primary, expected.primary);
  });
}

int _packedColor(Color accent) =>
    ((accent.a * 255).round() << 24) |
    ((accent.r * 255).round() << 16) |
    ((accent.g * 255).round() << 8) |
    (accent.b * 255).round();
