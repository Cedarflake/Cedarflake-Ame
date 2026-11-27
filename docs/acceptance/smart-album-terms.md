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
