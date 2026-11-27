# Prerelease compatibility cleanup

The [roadmap](../roadmap.md#prerelease-compatibility-cleanup) owns delivery status. The
[plan](../plans/prerelease-compatibility.md) scopes retirement;
[ADR 0002](../architecture/0002-rust-owned-catalog-and-media-boundaries.md#supported-prerelease-baseline)
owns the supported baseline.

## Retirement boundary — 2026-11-27

| Path | Disposition and purpose |
| --- | --- |
| Earlier default-data directories | Automatic import and fallback removed in the MSIX change; the workstation relocation was performed once outside the product. |
| Historical catalog upgrades and migration backup | Replaced with direct schema-83 initialization; existing current catalogs retain validation and recovery. |
| Storage settings v1 upgrade | Removed; empty settings start at v2 and unsupported existing data is rejected. |
| Model installation settings v1/v2 upgrades | Removed; empty settings start at v3, preserving current selection, download intent and GPU preference behavior. |
| Current synchronization and interrupted-operation recovery | Retained. Current-schema data can still contain unfinished or unowned work; rejection, retirement and bounded recovery prevent publishing it without authority. |
| Obsolete preview-file recognition | Retained for cache accounting and cleanup; these files are not promoted to current previews. |
| Broker protocol/version checks | Retained to reject obsolete or mismatched commands, without translating them into current commands. |
| Model/feature identities and archive versions | Retained as current reuse and import-admission contracts. |
| Catalog relocation and source-state checks | Retained as controls for current user operations. |

Startup wording in English, Simplified Chinese and Japanese describes an unsupported data format
and the available diagnostic-export action. The three user guides describe the common location,
retired Alpha conversion and retained normal-update behavior. Installer removal text is covered by
the [Store lifecycle record](store-msix.md).

## Independent baseline reference

Before replacing the initializer, the existing Debug library created a new isolated catalog through
the production `loadLibraryCatalog` bridge. Catalog source was unchanged between `3e8400f8` and
`eb54aa3a`. The capture verified the isolated preference/storage paths before opening the catalog;
it did not open the workstation's library.

The reference has schema/user version 83, zero roots and assets, a passing SQLite quick check,
310 non-internal schema objects and 21 populated initial tables. Ignored evidence is retained under
`build/prerelease-compatibility/before.json` and `before.sql`. The normalized reference schema digest
is `6350826383237dbb7de76e1b83dccf9b14a1dcf46ca5380e263f4a315570fa49`.
The exact schema-text comparison is a one-time replacement check, not a permanent source-text test.

The one-time comparison passed: all 310 schema objects and the 21 populated initial tables match
the pre-change reference. `baseline-comparison.json` retains the comparison under the same ignored
evidence directory. It enables foreign keys and applies the baseline inside `BEGIN IMMEDIATE` /
`COMMIT`; SQLite foreign-key and quick checks also pass.

## Current-format verification

The first schema run exposed a seed-order error: dispatch admission referenced the dispatch
singleton before its insertion. Applying the reference rows with foreign keys enabled identified
the required order. The subsequent 127-case schema run passed 125 cases and exposed two remaining
differences: alphabetic index creation changed SQLite's choice between equivalent indexes, and an
undo-history corruption fixture relied on removed downgrade cleanup. The baseline now retains
reference index creation order; the fixture corrupts the actual current history cursor. Both
focused reruns passed without changing the current query or history assertions.

Fresh initialization, unsupported/non-Ame database rejection, missing-marker rejection and
out-of-space transaction rollback passed four cases. Storage-settings reopen/rejection passed
three cases; model settings, intent retention and GPU preference behavior passed ten. The
dependency-boundary suite passed all 26 cases after the separate MSIX inventory correction.
The initial whole-crate check exposed unused historical helpers and imports; they were removed,
and the subsequent check and targeted compilation completed without warnings.

The three updated translations passed the existing 964-message localization check. Their generated
accessors were regenerated with the installed Flutter SDK. These are delivery checks; no test was
added to freeze the wording.

The additional 52 current-workflow fixtures passed: 50 in the first invocation and two full-scan
handoff cases in a scoped rerun. Those two initially failed before handoff at preview creation,
because the resolved temporary namespace differed from the configured path. Reusing the same test
binary through `Invoke-AmeAcceptanceScratch` with TEMP/TMP bound to its physical workspace directory
passed both, without product or assertion changes. The lease was released after process retirement.
The broad current-schema run and these focused reruns cover current validation, recovery, media
admission and handoff fixtures retained after removal of historical downgrade setup.

Logs under ignored `build/prerelease-compatibility` retain the failed attempts and focused reruns.
Scoped Rust formatting and diff checks passed.

Independent review of `2da71f0f` identified an existing connection-order side effect: opening a
DELETE-journal database switched its persistent mode to WAL before format rejection. WAL activation
now follows successful initialization and recovery. The retained rejection cases use the production
open path and compare complete fixture bytes before and after rejection. All four initialization
cases passed for this correction. The first invocation failed to create the default temporary
directories; the same compiled binary passed in the repository's leased scratch directory, which
was then retired. No record or validation assertion was removed.

### Hosted cleanup follow-up — 2026-11-27

The [full CI run for `5425a7c9`](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/45953339140)
failed in Static and the catalog/schema test jobs. Three remaining fixtures still required retired
v16, v17 or v23 migration behavior; those migration-only cases are removed under the supported
baseline decision. Current queue restart and generation-authority checks, payload-bound cross-root
lineage rejection, and byte-preserving unsupported-format rejection remain covered.

The query-plan failure came from SQLite choosing the equivalent classification-publication index
for a root/scan/source-generation lookup. The assertion now checks the indexed search and complete
key predicates, requires a covering index for the root keyset query, and continues rejecting a
table scan or temporary sorting tree. Queries and indexes are unchanged. The Static findings were
an empty line after an attribute and four single-element loops; both forms are removed without
changing validation assertions.

The scoped follow-up passed all-target/all-feature Clippy with warnings denied and 13 focused
tests covering repair admission, indexed keyset reads, initialization/rejection, current queue
restart and lineage ownership. Tests ran serially in the repository's leased scratch directory;
the lease was retired afterward. `ci-followup-clippy.log` and `ci-followup-focused-tests.log` retain
the results under the existing ignored evidence directory. Final-head hosted checks passed with
PR 76; the [Store integration record](store-msix.md#hosted-verification) retains the exact source
revision, CI run and merge-tree comparison.

## Implementation review boundary

Fresh initialization has 82 Rust lines and 140 dedicated test lines; its transaction applies a
2,897-line baseline SQL file containing the current schema and initial rows. The migration facade
is 100 lines; current validation is 282 lines, with 650 lines of retained schema definitions used
by structural validation. These owners have no inline tests. Storage initialization is 57 lines and
model-settings initialization is 74 lines, with their behavior covered in the existing adapter
suites. The baseline replaces historical construction; it adds no schema version or feature.
The affected `sqlite_catalog.rs` owner retains 2,534 production/structural lines and 379 test-support
lines, with no inline test cases. Its main dedicated test owner spans 36 files and 10,546 lines.
The former migration facade contained 4,212 production and 2,290 inline-test lines; current
validation and recovery remain in their existing scoped owners.
The hosted follow-up leaves `persistent_journal.rs` at 1,316 production and 2,825 inline-test lines;
the dedicated change-queue test owner is 6,032 lines. These two owners only lose the three retired
migration fixtures described above.
