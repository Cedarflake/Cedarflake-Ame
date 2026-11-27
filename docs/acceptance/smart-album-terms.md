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

## Ownership and reviewability

The new responsibility has separate domain validation, application commands, SQLite transaction,
bridge adapter and presentation owners. No schema migration or dependency change is required.
The existing runtime exports its already-admitted vocabulary/naming constructor for immutable bundled
defaults; the centralized composition register records that lifetime.

Physical Rust line counts at this checkpoint (production / inline tests / dedicated tests):

- `application/classification_configuration`: 378 / 0 / 198.
- `sqlite_catalog/classification/settings`: 1,403 / 0 / 4,750 across its existing modules.
- `application/classification_runtime`: 1,598 / 24 / 291 across its existing modules.

The new domain, application, defaults and SQLite owners are 119, 136, 49 and 65 lines respectively;
their dedicated application and SQLite term tests are 141 and 169 lines. No term-specific branches
were added to the discovery orchestrator or the existing settings transaction body.

After complete-directory activation, the domain, application and SQLite term owners are 138, 151
and 69 production lines, with no inline tests. Application and SQLite dedicated term suites contain
380 and 230 lines. Existing maintenance regressions own retained-origin protection.
