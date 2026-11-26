# R10 diagnostics and recovery guidance

Date: 2026-11-26

Scope: R10-04 in the [Beta readiness plan](../plans/r10-beta-readiness.md#r10-04--diagnostics-and-recovery-guidance).
The [roadmap](../roadmap.md#diagnostics-and-recovery-guidance) owns stage disposition;
[ADR 0044](../architecture/0044-local-diagnostic-export.md) owns the diagnostic-data and lifecycle boundary.

## Verification snapshot

The observations below apply to this implementation on `codex/r10-diagnostics`, based on
`7281949995313a752579053e77b34ad12631d754`. No exact-commit hosted result is recorded here yet.

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

## Documentation and remaining evidence

The English, Simplified Chinese and Japanese [user guides](../user-guide.md) cover import, models,
installed and portable versions, updates, organization backup/recovery and diagnostic sharing.
Their settings/action labels were checked against the localization catalogs. English and Japanese
gallery labels use the actual `All photos` and `すべての写真` strings. Each localized README links
to its guide; the documentation and acceptance indexes link to this record.

Remaining evidence: the applicable hosted checks for the published candidate.
R10-05 candidate and release qualification remain in their existing owners.
