# Smart-album term editing evidence

The [roadmap](../roadmap.md) owns delivery status; the
[term-editing plan](../plans/smart-album-terms.md) owns this scope.

## Focused verification — 2026-11-27

Seven native transaction/application cases passed for first-edit initialization without models,
custom create/delete, same-term conflict, preservation of unrelated definitions and dispatch state,
rename-only encoder/run retention, bundled reset and retained manual protection. The initial test
attempt could not create temporary SQLite storage under the sandbox's default temporary directory;
the same cases passed with temporary storage below repository `build/test`.

Twenty-two Flutter cases passed across term management, existing album forms/pickers, smart-album
navigation and production bindings. Term checks use the application theme in English, Simplified
Chinese and Japanese at 150% text scale. They cover immediate persistence, reset/deletion, retained
tray position, duplicate-submit/dismissal prevention, retained failed drafts and successful saving
after reopening a concurrently changed term. Existing album selection, creation, pagination,
cancellation and commit-before-publication checks remain passing after sharing the tray shell.

Scoped Dart analysis, Rust format, localization-source validation and dependency boundaries passed.
The boundary checker inspected 673 Dart modules with zero frozen migration exceptions. Generated
bridge output comes from the repository's bridge generator; the new generated API belongs to its
exact roster. The existing cached-analysis regression passed with zero decode/inference calls.

## Native client workflow

`integration_test_classification.ps1 -Scenario terms` uses existing verified local model/runtime
artifacts and an isolated Debug catalog. The reusable fixture has three images with two distinct
contents plus an existing diagnostic script. The first attempt stopped before scanning because it
assumed exactly three files; the fixture remains intact, and the workflow now checks all original
files and the unchanged directory entry count after classification shutdown.

The completed Windows workflow passed create/edit/reset/delete through the real forms and native
bridge. Before selecting a model, a renamed starter term persisted and the full six-purpose starter
taxonomy was initialized. After scanning, the first two-content classification completed in 9,685 ms;
description-only rematching completed in 4,230 ms. Those are fixture observations, not large-library
performance targets. All pre-existing feature keys/checksums remained and one text feature was added.
The owning cached-analysis regression separately verifies zero decoding/inference on cached input.
After worker shutdown, all four original files and the directory entry count were unchanged.

An earlier native attempt reached successful matching/reset/create but its test tried to address an
unbuilt offscreen row. The final interaction scrolls the lazy list to the new term before reopening
and deleting it. The completed run took 23 seconds after the 37.9-second incremental Windows build.

## Editor presentation and complete directory — 2026-11-28

The term tray places its add action in the header and uses framework dismissal. The editor groups
filled text fields, retains indexed accessible names, and moves the English-language suggestion to
the existing tooltip owner. Its title remains outside the content scroll view. Reset no longer
changes the Save caption or re-enables controls while the successful route is closing.

The installed `material_ui` 1.11.1 supports `InputDecoration.filled` and the standard
`AlertDialog(scrollable: false)` with a `SingleChildScrollView` body, matching the official
[input decoration](https://api.flutter.dev/flutter/material/InputDecoration-class.html) and
[dialog scrolling](https://api.flutter.dev/flutter/material/AlertDialog-class.html) APIs. These
framework capabilities cover this form without a custom input or dialog layout. The sidebar edit
action now uses the existing 36-pixel action bounds and an 18-pixel glyph.

All seven term-editor cases passed, including accessible-name, closing-route admission and
unconfigured-term draft reset/save activation assertions. The first accessibility-check attempt
released its semantics handle in test teardown, after Flutter's end-of-test invariant; scoped
disposal before the test ends passed.

The updated Windows workflow passed through the real forms and native bridge in 33 seconds after
a 20.3-second incremental build. Initial classification took 9,434 ms; activating a dormant bundled
term took 4,156 ms and retained all existing feature payloads. A description edit took 4,154 ms,
retained those payloads and added one text feature. Create/edit/reset/delete passed with all four
source files unchanged. The first invocation used the model parent directory instead of its verified
leaf and stopped at model selection; the corrected invocation reused the completed native build.

An isolated normal-client preview confirmed the smaller sidebar edit action, header add action and
complete directory. A seven-description draft scrolled while its title and actions stayed fixed;
restoring an unconfigured draft returned to its bundled description without closing the editor,
and Save returned to the retained term list. The preview used disposable catalog storage.

The directory combines all 56 visible bundled definitions with current configured terms. Reading
does not write a taxonomy. Saving an unconfigured definition activates that term and reuses the
existing semantic-change lifecycle; the first save additionally installs the basic purpose set.
Configured name-only edits retain the analysis run. Expected-state comparison uses actual stored
definitions before starter initialization, so unrelated edits merge and same-term changes reject.
Fifteen focused native cases passed for read-only directory loading, first activation, invalid IDs,
same-term conflicts, unrelated changes, retained model/dispatch state and protection after reset.
The existing cached-feature case again performed no decode or inference. Bridge verification passed
all 39 asynchronous contracts and matching content hashes.

## Bulk defaults and directory filtering — 2026-11-28

Ten focused Flutter checks passed for the term workflow, including source filters retained across
editing, restoration across the hidden built-in rows while Custom is selected, preserved custom and
dormant definitions, cancellation, pending dismissal/duplicate-submit prevention, failed-batch retry,
and name-only/no-op lifecycle behavior. The existing lower-row editing case now sends a real pointer
scroll event: it observes an intermediate deceleration position before the exact final distance,
then verifies position retention after editing. The tray and wrapper share one `AmeScrollController`.

`material_ui` 1.11.1 supplies the single-selection `ChoiceChip` used for directory filters, matching
the official [single-selection filter guidance](https://api.flutter.dev/flutter/material/FilterChip-class.html).
Origin labels use existing list typography; the restoration action uses the shared popup menu and
confirmation dialog. Localization verification passed 1,043 messages and the unchanged 39-contract
bridge checks passed with matching generated hashes.

The normal Windows preview also shows compact filters with an 18-pixel, weight-300 Material Symbols
check in the native chip avatar slot. The pinned chip implementation derives its painted check's
stroke from avatar height rather than icon weight; its public avatar and `showCheckmark` properties
allow the existing Ame glyph without replacing selection or focus handling. Empty and populated
filters retain the same 360-logical-pixel content region, constrained by available window height;
switching to the empty Custom filter keeps the header in place and centers its message. The first
attach/reload retained the prior layout; a full hot restart and subsequent attached reload displayed
the current layout and glyph.

The initial widget assertion compared a custom definition's object identity across an intervening
Save; it now compares its ID, name and descriptions. The initial 18-case Rust run passed 17 cases;
one SQLite historical-setup assertion incorrectly expected old display names. Loading intentionally
combines current labels with versioned descriptions. The corrected check retains exact run, rule
revision, refinement, category-ID and description assertions while checking current display names.
Targeted runs passed the corrected mixed-batch case and the maintenance-protection case, giving
19 applicable passing cases across the initial and targeted runs.

The Windows terms workflow passed in 44 seconds after a 359.2-second native rebuild. It restored
two modified built-ins through the confirmation while the Custom filter was selected, retained
the custom definition and exact configured ID set, and completed matching in 2,867 ms using all
previous feature payloads. This is a small-fixture observation rather than a performance target.
Individual reset, custom deletion and dormant activation remained passing; all four original files
and the directory entry count were unchanged after shutdown.

## Ownership and reviewability

The new responsibility has separate domain validation, application commands, SQLite transaction,
bridge adapter and presentation owners. No schema migration or dependency change is required.
The existing runtime exports its already-admitted vocabulary/naming constructor for immutable bundled
defaults; the centralized composition register records that lifetime.

Current physical Rust line counts (production / inline tests / dedicated tests):

- `application/classification_configuration`: 440 / 0 / 552.
- `sqlite_catalog/classification/settings`: 1,457 / 0 / 5,187 across its existing modules,
  including dedicated coverage tests beside the adapter owners.
- `application/classification_runtime`: 1,598 / 24 / 291 across its existing modules.

The per-term and batch-reset domain owners contain 147 and 98 production lines. The application,
defaults and SQLite term owners contain 198, 49 and 119 production lines, with no inline tests.
Dedicated application and SQLite term suites contain 495 and 507 lines, including 125 application
and 273 SQLite batch-test lines. Existing maintenance regressions own retained-origin protection.
No term-specific branches were added to the discovery orchestrator or the existing settings
transaction body.

The tray, reset dialog and Dart application owner contain 291, 79 and 165 production lines; the
widget suite, shared fixture and native workflow contain 443, 182 and 299 lines.
