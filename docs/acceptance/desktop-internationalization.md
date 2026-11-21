# Desktop language verification

## Windows client, 2026-11-21

The [desktop language workflow](../../test/support/desktop_language_workflow.dart) runs in the
[application widget test](../../test/app/desktop_language_workflow_test.dart) at 960 × 700 and in
the [Windows scan integration](../../integration_test/scan_workflow_test.dart) after importing a
disposable folder through the native picker.

On Windows x64 with Flutter 3.50.0, all four native integration cases passed. The client selected
a photo, switched from Simplified Chinese to English in Settings and returned to the same selected
photo. A synchronization issue recorded before the switch appeared in English. Opening its update
action selected the original folder, including its Chinese path. The client returned to Chinese
and the test compared the fixture files byte for byte. The widget test also checks that switching
languages issues no additional gallery query.

Both [native accessibility cases](../../integration_test/windows_accessibility_bridge_test.dart)
passed on the same x64 client. Windows UI Automation confirmed ten checkpoints, including opening
and closing folder, viewer and settings menus. The gallery fixture builds its dated timeline after
the application's localization delegates have loaded.

The x64 test used a process-local `CARGO_PROFILE_DEV_DEBUG=0` for the Rust Debug dependencies after
MSVC failed to write their debug records. The Flutter client ran in Debug mode. ARM64 native
execution and translated MSI dialogs are outside this client run.

## Catalogs and preferences

The [locale tests](../../test/app/ame_localizations_test.dart) cover ordered system preferences,
English fallback, singular and plural counts, date formats and the distinction between shipped
category labels and stored names. The [preference adapter tests](../../test/features/settings/adapters/platform/shared_preferences_ame_store_test.dart)
cover the system default, malformed values and explicit choices. The native workflow reloads the
preference store after changing the language in either direction.

The original English and Simplified Chinese catalogs contained 913 messages. The presentation gate
regenerated them, checked formatting for 959 Dart files, validated 629 module boundaries and passed analysis. Negative fixtures rejected
missing translations, empty values, dropped or undeclared parameters, hardcoded Chinese and an
English widget label. A focused 86-test run covered viewer timestamps down to the second, settings,
update feedback and retained notifications. The filter interaction also passed using the
Chinese date picker's input format and confirmation button.

## Japanese and script-specific Chinese, 2026-11-21

The three languages each resolve 914 messages. The catalog gate rejected Japanese fixtures with
a missing key, empty text, a missing argument or an unknown key. The source checker rejected a
hardcoded katakana label. The three centered README selectors linked to the other two languages.

The four Windows x64 scan integration cases passed. In the populated workbench, the selected
photo survived the switch from Simplified Chinese to English and then Japanese. The same earlier
notification opened the original folder in both languages. The workflow returned to Simplified
Chinese and compared the original fixture bytes. The focused widget and adapter suites also
checked Japanese dates and counts, system changes from `zh-CN` to `ja-JP`, explicit scripts with
conflicting regions, and saving and reopening the four language choices.

Two separate Flutter test processes used the production Windows preference adapter with an
isolated on-disk directory. The first saved `ja`. The second loaded that choice with an English
system locale and displayed the workbench's language setting and startup recovery in Japanese.
This client evidence covers Windows x64.

## Affected implementation owners

The message types and their presentation resolver contain 252 and 226 production lines respectively.
The gallery screen has 2,034 lines and delegates settings-return selection to its 163-line workspace
coordinator. These owners contain no inline tests. The coordinator's dedicated suite has 322 lines.
The widget language test and its shared workflow have 46 and 129 lines and exercise the integrated
path through the 588-line native scan suite.

The Japanese extension brings the shared language workflow to 153 lines. The Settings page has
319 production lines and a 1,281-line widget suite. The preference adapter has 251 production
lines and a 218-line dedicated suite. Both production owners have no inline tests.
