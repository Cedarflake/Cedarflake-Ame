# Internationalization

## Changing interface text

Update all supported languages in the same change as the feature. This includes menus, tooltips,
accessibility labels, notifications, startup errors and labels Ame supplies to native dialogs.
Use the existing Material translations for controls whose wording the framework owns.

1. Add or reuse a key in the [ARB catalogs](../../lib/app/presentation/l10n). Name the key for
   its meaning and use. Two actions with the same wording can need different translations.
   Put Simplified Chinese in `app_zh.arb`, English in `app_en.arb` and Japanese in `app_ja.arb`.
   Flutter's `app_zh_Hans.arb` inherits the Chinese template.
2. Translate the whole message so each language can choose word order, punctuation and plural
   forms. Declare named, typed placeholders in the template and add a translator description when
   the context is ambiguous. Use ICU plurals where counts change the wording, and format numbers
   and dates in the active locale. For example, `context.l10n.photoCount(count)` lets English
   distinguish “1 photo” from “2 photos”.
3. Run `flutter gen-l10n` with the [pinned Flutter SDK](../acceptance/quality/execution-safety.md#local-flutter-toolchain).
   Use the generated getter or method through `context.l10n` in widgets. Edit the ARB sources
   and commit the regenerated Dart files alongside them.

## Keeping data independent of the display language

Store task feedback and notification history as a typed [AmeMessage](../../lib/app/messages/ame_message.dart)
with its arguments. Resolve the text when displaying it so an existing entry follows a language
switch. Use stable IDs for actions and Rust error codes for localized recovery guidance.
Keep the original diagnostic available in technical details.

Preserve filenames, paths, metadata and user-entered names verbatim. For built-in categories,
translate the [shipped label](../../lib/features/library/presentation/library_category_label.dart)
only while the category still has its original ID and default name. Preserve renamed and
discovered labels. Model descriptions are classification inputs and affect the semantic revision.
Keep them outside the translation catalogs.

## Language tags and new languages

Use BCP 47 tags for explicit language preferences and translated README filenames. `zh-Hans`
and `zh-Hant` identify writing systems. `zh-CN` and `zh-TW` include regions. The `system`
preference follows Windows. Honor an explicit script when resolving Windows preferences.
The [application language rules](../product/gallery-ui.md#scope-and-authority) define the
supported choices, preference order and fallback. Only add a preference migration for a value
that a released version actually stored.

When adding a language, supply its catalog, register it in locale resolution and Settings, and
show its name in that language. Extend the catalog check and the language-switch tests to cover
the new choice, including startup and restoring the saved preference.

When shared README content changes, update the [English](../../README.md),
[Simplified Chinese](../../README.zh-Hans.md) and [Japanese](../../README.ja.md) editions
together. Keep the language links centered below the logo, with the current language in plain
text and relative links to the other editions.

## Verifying a change

Run `./tool/quality_verify_localizations.ps1` for a focused check of keys and placeholders.
The [presentation lint gate](../acceptance/quality/commands.md#repository-command-reference)
also catches stale generated files and hardcoded text in common UI controls. Exercise the affected
screen in each supported language. Check that longer labels fit in a narrow window and with
enlarged text, and that icon-only actions have localized accessible names.
Treat fixed text columns and readout widths as potential clipping points: let labels and values
share content-aware columns with an explicit gap, keep numeric readouts on one line, and reflow
forms when their fields cannot show complete values. Validation messages must wrap in the actual
input decoration. Keep fixed icon/target sizes and ordinary spacing tokens when they remain usable.
For affected controls, check all supported languages at the 1100 × 700 desktop minimum and 150%
text, including open menus, validation failures and the connected action. A layout that only fits
its default language or empty values is insufficient.
Widget window fixtures set the test view's physical size and pixel ratio together so layout
constraints and `MediaQuery` describe the same logical window.
For changes to language switching, follow the [desktop language workflow](../acceptance/quality/ui-verification.md#ui-workflow-verification)
to verify selection, saved preferences and actions on earlier notifications.
