# R10 diagnostics and recovery

Date: 2026-11-26

Scope: R10-04 in the [Beta readiness plan](../plans/r10-beta-readiness.md#r10-04--diagnostics-and-recovery).
The [roadmap](../roadmap.md#diagnostics-and-recovery-guidance) owns stage disposition;
[ADR 0044](../architecture/0044-local-diagnostic-export.md) owns the diagnostic-data and lifecycle boundary.

## Verification snapshot

The observations below apply to implementation `3a48aadb135d7746894fab1f28a91773ea853816` on
`codex/r10-diagnostics`, based on `7281949995313a752579053e77b34ad12631d754`.
[Quality / CI run 45749814596](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/45749814596)
passed the full checks for that commit in [PR 75](https://github.com/Cedarflake/Cedarflake-Ame/pull/75).

| Check | Observed result |
| --- | --- |
| Focused Flutter checks | 28 cases passed across diagnostic application, adapter, presentation and bootstrap coverage, with affected update/settings checks |
| Dart analyzer | No issues found |
| Feature dependency boundaries | 663 modules, zero exceptions |
| Windows x64 Debug builds | Successful initial build in 146.7 seconds and subsequent build in 27.2 seconds; these are build durations, not application performance measurements |
| Settings export in a real empty-library client | Native save dialog completed and the ZIP was created |
| Native save cancellation and another export | Cancel returned to the client; another export completed successfully |
| Export contents | Exactly `summary.txt` and `diagnostics.json`; inspected output contained no personal paths, names or credentials |
| Startup-failure desktop export | A client launched with a temporary damaged SQLite catalog displayed its failure page and exported the diagnostic ZIP |

The application checks exercise native-picker cancellation, cancellation before collection completes,
destination failure and retry, open-folder failure recovery, shutdown joining active writing, and a
shared diagnostic controller surviving provider-container recreation. Adapter checks decode a real
ZIP, preserve a prior destination when cancelled, reject inadmissible values and retain the newest
256 journal events across restart within 128 KiB. Startup-event persistence checks cover delayed journal
initialization; unavailable journal storage retains in-memory evidence.

Projection checks keep typed application and native error identifiers while excluding raw messages,
paths and exception stringification. The library snapshot distinguishes GPU preference from observed
CPU execution, retains model selection independently of the installer task, and reads idle exact and
similar analysis state without starting analysis. UI checks cover localized settings and feedback.

The startup-failure interaction used the latest 27.2-second Debug build and a temporary damaged
SQLite catalog supplied through `Invoke-AmeAcceptanceScratch`. The real failure page exported
`summary.txt` (741 bytes) and `diagnostics.json` (1,100 bytes). The payload included
`startupState: failed`, supported schema versions 83 and 2, `application_initialization_failed` and
`catalog_database_error`, without the original path, media or credentials. Its existing bottom
feedback offered Open location and acknowledgement; Open location opened the chosen build output
directory. The temporary Explorer window and client were closed, the client exited with code 0,
and the scratch fixture was cleaned up.

A separate missing-DLL probe exited with code 1 before the Dart interface and produced no log.
It did not exercise the failure page. The temporarily moved Debug dependency was restored in
`finally`, and its SHA-256 matched the original. Diagnostics export depends on reaching the Dart
startup/fallback interface; this check does not establish recovery from a process-loader failure.

## Environment fields — 2026-11-26

The environment extension adds CPU model, OS-usable physical-memory bytes, installed display-adapter
models with their corresponding driver versions, Flutter framework/engine build identity, and the
current window's display scale. Format 2 retains the existing two-file ZIP layout and bounded journal.
The native hardware query runs on one worker independently of catalog startup; unavailable hardware
does not remove OS/runtime facts or prevent export.

Four environment cases passed for paired drivers, 125% → 150% scale sampling, manufacturer model
punctuation, missing/native-failed collection, a two-second deadline, and partial malformed values.
The six existing platform-export cases also passed with format-2 adapter assertions. The affected
Dart analyzer and format checks passed. The Windows Debug client built successfully in 29.1 seconds.

Settings/About exported a real format-2 ZIP: `summary.txt` contained 2,174 bytes and `diagnostics.json`
contained 3,672 bytes. CPU model, all three reported display devices and their driver versions matched
independent Windows CIM queries. Physical-memory bytes matched CIM's total visible memory converted
from KiB, and the window reported 125% scaling. Framework 3.50.0 and engine `371f77202a` matched the
build's SDK defines. Collection retained the safe prior events and contained no personal paths or
computer names. This desktop run covers the hardware channel and export; existing startup-failure
coverage above remains applicable to the shared collector.

Affected ownership counts are 123 production lines for the Dart environment adapter, 158 for its
Windows adapter (plus a 24-line declaration), 99 for ZIP encoding, 207 for bootstrap bindings, and 174
for runner composition, with no inline tests. Dedicated environment tests contain 151 lines; the
shared platform-export suite contains 223. Hosted checks cover the pushed revision separately.

## Source and storage boundaries

The desktop verification used an empty-library client. No original media was modified or hydrated.
ZIP export writes only the selected report and its owned temporary sibling. The journal uses
application-support storage. No diagnostic upload is performed.

The SQLite catalog and settings adapters only change their schema-constant imports. The values
remain catalog version 83 and settings version 2; migration SQL and existing catalog behavior are
unchanged. Exported versions describe the versions supported by this build, not an inspection of
an unavailable database. Generated bridge files expose those constants without opening a catalog.

## Physical ownership

Counts below are physical lines at this working-tree checkpoint, including blank lines. Dedicated
tests may exercise more than one owner; their counts are not counts of newly added tests.

| Owner | Production | Inline tests | Dedicated test owner |
| --- | ---: | ---: | --- |
| Diagnostic export controller | 223 | 0 | Application lifecycle suite: 242 |
| Diagnostic journal | 147 | 0 | Shared platform adapter suite: 205 |
| Diagnostic ZIP/platform adapter | 99 | 0 | Shared platform adapter suite: 205 |
| Diagnostic feedback | 107 | 0 | Presentation suite: 153 |
| Bootstrap diagnostics bindings | 205 | 0 | Provider recreation in the application lifecycle suite |
| Library snapshot projection | 174 | 0 | Snapshot suite: 129 |
| Error projection | 66 | 0 | Error-projection suite: 104 |
| Startup-failure surface | 143 | 0 | Startup-failure suite: 123 |
| Application startup composition | 233 | 0 | Existing startup/fallback checks and diagnostic lifecycle suite |
| SQLite catalog adapter | 2,538 | 0 | `sqlite_catalog/tests.rs` and `sqlite_catalog/tests/`: 12,337 |
| SQLite settings adapter | 246 | 85 | No separate test file |

The catalog adapter's 2,977 physical lines comprise 2,538 production lines and 439 lines gated by
`#[cfg(test)]`, including helper functions, instrumentation, imports and module declarations. It
contains no inline test cases. The dedicated count includes its 27-line `tests.rs` module index
and 37 Rust files under `tests/`, including fixture support. This change relocates one constant
rather than adding workflow behavior. The settings adapter's 85 inline-test lines include the
test-module declaration. Neither adapter gains SQL or test cases in this slice.

## Remaining desktop evidence

The subsequent startup-failure action layout places Export and Copy side by side in a centered
group, retaining wrapping for constrained text/window sizes. Its Windows Debug build completed in
21.5 seconds, and the actual damaged-catalog startup page displayed both buttons on the same row.
This layout follow-up retains its separate exact-commit hosted result; the run above covers the
preceding implementation.
R10-05 candidate and release qualification remain in their existing owners.

## Presentation test maintenance

The final copy change exposed four stale title lookups in
[CI run 45760537997](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/45760537997).
The affected five cases passed after aligning the title lookup. The subsequent test cleanup removes
34 cases that freeze copy, visual constants or one-off alignment, and removes incidental style
assertions from mixed workflow tests. Dedicated visual fixtures and their unused helpers are removed.
The [test-design standard](../development/testing.md#temporary-presentation-checks) now assigns this
work to temporary desktop verification.

Retained coverage includes empty-state publication and import admission, sidebar reversal and reduced
motion, gallery row continuity, menu keyboard interaction, notification actions, settings persistence
and recovery, preview-demand retirement, and text contrast. No production behavior changes accompany
this cleanup. Formatting, affected-file analysis and the final diff were checked locally; the hosted
suite results are recorded below.

## Integration — 2026-11-26

[PR 75](https://github.com/Cedarflake/Cedarflake-Ame/pull/75) was merged into `main` as
`1cfbca7c6830d909a0ce29a641bbbcc91d14d8c0` after
[Quality / CI run 45763476904](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/45763476904)
passed for final head `84136074af584ebdc750945f48cc37219c8c023e`.
All selected static, Flutter, Rust, Windows scan, accessibility and production-build checks passed,
including the aggregate Windows Gate. The preceding failed copy lookups and their disposition remain
recorded above.

The merge used base `7281949995313a752579053e77b34ad12631d754`. The merged tree and checked head both
resolve to `3259013f99f86ebc62babca463d85092de9e9179`; the retained desktop evidence therefore applies
within its stated scope. Local `main` was fast-forwarded to the merge, and `codex/r10-diagnostics`
was deleted locally and remotely. R10-05 candidate qualification and publication retain their
separate scope in the roadmap.
