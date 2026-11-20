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
exercise older stores, malformed values and explicit choices. The native workflow reloads the
preference store after changing the language in either direction.

Both catalogs contain 913 messages. The presentation gate regenerated them, checked formatting for
959 Dart files, validated 629 module boundaries and passed analysis. Negative fixtures rejected
missing translations, empty values, dropped or undeclared parameters, hardcoded Chinese and an
English widget label. A focused 86-test run covered viewer timestamps down to the second, settings,
update feedback and retained notifications. The filter interaction also passed using the
Chinese date picker's input format and confirmation button.

## Affected implementation owners

The message types and their presentation resolver contain 252 and 226 production lines respectively.
The gallery screen has 2,034 lines and delegates settings-return selection to its 163-line workspace
coordinator. These owners contain no inline tests. The coordinator's dedicated suite has 322 lines.
The widget language test and its shared workflow have 46 and 129 lines and exercise the integrated
path through the 588-line native scan suite.
