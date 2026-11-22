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
The gallery screen has 2,040 lines and delegates settings-return selection to its 170-line workspace
coordinator. These owners contain no inline tests. The coordinator's dedicated suite has 354 lines.
The widget language test and its shared workflow have 46 and 163 lines and exercise the integrated
path through the 588-line native scan suite.

The Settings page has 319 production lines and a 1,394-line widget suite. Its storage section has
775 production lines and delegates error presentation to a 57-line widget. Neither has inline tests.
The preference adapter has 251 production
lines and a 218-line dedicated suite. Both production owners have no inline tests.

## Language round trip, 2026-11-21

At `5a93706b`, selecting a photo and changing Chinese to English and back to Chinese during
one Settings visit cleared the selection on return. The workspace now tracks language changes
through the visit in the 14-line `LibrarySettingsVisit` owner. Its state is reset on the next visit.
The shared widget/native workflow exercises this round trip before its existing notification
actions. The application widget case and all 17 workspace coordinator cases passed locally on
Windows x64, including ordinary Settings return, failed navigation and disposal.

## Storage configuration feedback, 2026-11-21

The storage section retains structured configuration failures until presentation. The
`storage_path_overlaps_source` rejection has Chinese, English and Japanese guidance, with the
original code and diagnostic available through an expandable details button. The settings widget
workflow rejects a location inside a photo folder, switches all three languages without another
save, opens and closes the diagnostic, then selects another location and completes the save.
All 17 settings cases passed locally on Windows x64. The catalog gate checked 917 messages per
language. Native path validation and storage persistence are unchanged.

At `0ff1f3da`, hosted Static rejected the new storage-error widget's undeclared domain dependency.
Its exact `StorageSettingsFailure` consumer is now registered under ADR 0037, with the Dart import
limited to that type. The local dependency gate passed all 632 modules with zero migration
exceptions; all 19 boundary-rule cases and 17 settings cases passed.

## Sidebar action sentences, 2026-11-21

Sidebar section toggles use complete localized messages with a typed section-name argument.
The three locale cases in the 46-line sidebar toggle suite passed locally, checking both tooltip
and accessibility tooltip text before and after collapse and expansion. English retains the
space before the section name, and Japanese places the section before the action.

## Localized rail geometry, 2026-11-21

The language workflow reproduced the background discontinuity at `98329662`: English expanded the
rail to 141 logical pixels in the widget fixture while the chrome still started at 72. The frame
now lays out the rail and hamburger in one column beside the chrome. The installed Material rail
keeps its label sizing, selection and accessibility behavior; no duplicate text measurement is used.

Nineteen focused cases passed across the language workflow, workbench retention, accessibility,
duplicate navigation and shell layout. The language cases check the adjoining backgrounds,
hamburger centerline and gallery search alignment in all three languages at 100% and 150% text
size. One combined run stopped with a Dart allocation failure; the interrupted five-case shell
suite passed when run separately. The dependency gate passed all 632 modules without exceptions.
The rebuilt x64 Debug client showed continuous backgrounds after English, Japanese and Chinese
switches in Settings and after collapsing and expanding the English gallery sidebar. The preview
was left open in its original English setting. ARM64 client execution remains outside this local check.

The layout owners contain 74 production lines in `library_workbench_frame.dart` and 179 in
`library_global_bar.dart`, with no inline tests. The 356-line workbench and 2,034-line screen only
change header composition and its callback. Dedicated workbench and screen suites contain 231 and
373 lines; the shared language workflow and its widget entry contain 181 and 50 lines.

## Information tray columns, 2026-11-21

The information tray uses the installed Flutter `Table` with one intrinsic label column and one
flexible, selectable value column. Eleven bounded metadata fields share a 16-pixel column gap.
The three language cases passed at 800 × 560 with 150% text, checking separation, aligned values
and the unchanged source path. The original layout failed the separation check with a zero-pixel
gap. A rebuilt x64 Debug client also displayed the English camera fields and timestamps in aligned,
separated columns. The owner contains 157 production lines and its focused suite contains 61 lines.

## Action tooltip dismissal, 2026-11-21

The Windows tooltip now retires its pending timer and visible entry on pointer press. The original
favorite regression left its old hover text visible after pressing the button. Thirteen focused
cases now pass for favorite save/refresh, re-entry, pending hover, touch long-press, retained
surfaces, semantic labels and dialog focus. The rebuilt x64 client showed the hover disappear on
favorite activation and stay absent after the committed state refreshed. Accessible action labels
remain available. The overlay owner has 398 production lines and no inline tests; its lifecycle and
favorite suites contain 238 and 126 lines.

## Viewer notification acknowledgement, 2026-11-22

Persistent analysis failures without a recovery action now expose the shared acknowledgement
button in the viewer's existing action slot. Five focused cases passed: acknowledgement in all
three languages removes the visible notice while retaining its active historical condition,
undo results expire after five seconds, and failed saves retain their connected retry action.
The checks publish through the classification lifecycle and render the actual viewer notice
selector; acknowledgement does not mark the underlying analysis as recovered.

## First filmstrip selection, 2026-11-22

The new cold-selection regression failed before the repair: the viewer's first frame contained none
of the selected strip card's decoded pixels. Adjacent navigation now reuses the mounted card's image
provider and sampling quality until the guarded original arrives. Thirty-seven focused cases passed
for first-frame reuse, source replacement, retry, preview repair, strip paging and viewer return
transitions. The first-frame case also rejects an incompatible provider and verifies that strip
registration cannot replace the gallery return target and is removed on disposal.
The rebuilt x64 client displayed the selected strip thumbnail immediately, then replaced it with
the decoded original while keeping the strip and image bounds stable.

The launch/thumbnail owner contains 306 production lines, the viewer image 296 and the filmstrip 384,
with no inline tests. The viewer image suite contains 688 lines; the existing transition and strip
suites retain their prior behavioral assertions.

## Localized dimensions and spacing, 2026-11-22

The fixed-dimension review covered workbench navigation, Settings, filter controls, metadata rows,
viewer chrome/readouts, notices and shared dialog/form controls. Icon targets, scroll placeholders,
bounded thumbnails and spacing tokens retain their existing sizes. The normal window minimum is
1100 × 700; earlier 800 × 560 tray checks are additional narrow-window evidence.

At 150% text the review reproduced a truncated English date-type choice, a multi-line 1600% zoom
readout and clipped album-name validation messages. The date selector and its two date fields now
use separate full-width rows. Zoom text has a minimum width and remains on one line. Album and
filter validation messages wrap through the framework error widget instead of a one-line string.

The three-language layout cases check selected dates, date-type and unit menu actions, invalid
numeric input, zoom/count separation, search text height and notice acknowledgement. Settings
checks scroll through the real choices and select a language; album checks display validation and
save an entered name through the existing fixture. The focused suites passed 9 layout cases,
20 Settings cases and 9 album cases. The four existing filter workflows also passed after the date
layout changed. These are bounded widget checks, not exhaustive visual coverage of every string.
The rebuilt x64 client showed the English date selector, complete date fields and footer actions
inside the existing tray. The three new/extended layout suites passed focused static analysis;
the dependency gate passed all 632 modules with zero migration exceptions.

The filter, toolbar and album editor contain 347, 295 and 216 production lines, with no inline
tests. The layout, Settings and album suites contain 252, 1460 and 466 lines respectively.

## Hosted comparison fixture, 2026-11-22

The Flutter job at `d9d0f0f5` failed to find the ignored-group navigation in the comparison workflow.
The local reproduction measured an 1100 × 850 layout surface but an 800 × 600 `MediaQuery`, which
kept its sidebar collapsed. The fixture now sets the test view's physical size and pixel ratio
together and checks the resulting logical size. The new localized layout cases use the same view
setup. All 39 cases in the four affected suites passed, including the complete comparison workflow;
the widget assertions and product navigation behavior remain in place.

The complete Flutter job at `951b5342` exposed the same stale view dimensions in two remaining
duplicate-review fixtures: four wide-window layout cases and the preferred-copy/ignored-group
workflow. All five failures reproduced locally. Both fixtures now set and restore the test view's
size and pixel ratio, and assert the logical size before exercising the page. All 17 cases in these
two suites passed with their original layout, persistence, navigation and cleanup assertions.
The suites contain 416 and 326 lines; product code and CI selection are unchanged. The second failed
job in [run 44944759629](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/44944759629)
was the aggregate Windows gate reporting the Flutter failure.

## Integration, 2026-11-22

[PR 73](https://github.com/Cedarflake/Cedarflake-Ame/pull/73) integrated head `af6f0a66` into `main`
as `22816d57`. The merged file tree matches that head. Its
[complete CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/44973054453) passed the
selected static, Flutter, Rust, native scan/accessibility and Windows build checks before merge.

The [Flutter GIF playback regression](material-ui-migration.md#windows-gif-playback-regression-2026-11-22)
remains a separate runtime follow-up on `codex/track-flutter-gif-crash`; integration does not claim
that crash is repaired. Existing local client coverage and its ARM64 execution limit remain above.
