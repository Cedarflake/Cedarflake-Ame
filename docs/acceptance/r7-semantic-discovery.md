# R7 implementation evidence

Date: 2026-11-07
Scope: composed filters, CI reduction and shared-feature foundations on `codex/r7-semantic-discovery`.
The [roadmap](../roadmap.md#r7-implementation-sequence) owns disposition and the
[plan](../plans/r7-semantic-discovery.md) owns remaining slices.

## Composed filter implementation

The existing filter menu retains its four quick choices and adds `更多筛选`. A typed presentation
draft validates extensions, exact byte conversions, dimensions, explicit calendar dates and camera
text. Applying publishes one existing metadata query; cancelling preserves it. A retired opening
query cannot receive the editor result. Invalid fields are brought into view, including a collapsed
camera section. Header summaries remove one condition or clear metadata without changing scope,
literal search or duplicate display.

No schema, bridge, model package or source-write behavior changes. Native shared predicates,
query-generation handling and gallery position retention remain the existing owners.

## Component admission

Inspected the [Material component catalog](https://m3.material.io/components) and the installed
Flutter 3.47.8 / Dart 3.13.7 implementation; the hosted Windows workflows pin the same Flutter version.

| Purpose | Framework capability and choice |
| --- | --- |
| Draft editing | `AlertDialog`, `Form` and `TextFormField`; framework dismissal, focus traversal and field validation. `validateGranularly` identifies the field to reveal through framework scrolling. |
| Date field and size unit | `DropdownMenu` supports typed entries and `selectOnly` in the installed SDK. Existing menu and semantics boundaries remain shared. |
| Calendar | `showDatePicker` supplies a compact desktop dialog. A fullscreen date-range picker would not fit this interaction. Two independent bounds also support an open-ended interval. |
| Secondary camera field | `ExpansionTile` with its framework controller; invalid hidden input expands before scrolling into view. |
| Applied conditions | `InputChip` supplies deletion and accessibility. `TextButton` clears conditions. No custom input, menu, picker, scrolling or chip primitive was introduced. |

## Verification

- Focused Flutter checks: 16 passed across the filter editor, draft validation and existing position
  regressions. Coverage includes cancel, clear, individual removal, retired query authority, exact
  values above the floating-point integer range, invalid bounds/dates, and original-position recovery
  in both gallery layouts. Existing position cases now include metadata summary insertion/removal.
- Changed-file Dart analysis passed with no issues after the field-reveal change. Dart dependency
  boundaries passed for 529 modules with zero frozen exceptions.
- Built and ran the Windows Debug client with the production entrypoint and isolated preferences.
  Rust used the repository's optimized Debug profile with test-storage admission; the guarded build
  receipt bound source-input and native DLL hashes before launch.
- Reused 51 retained fixture images in a leased isolated catalog. The native client showed the
  standard filter menu, calendar and keyboard-editable fields. The following counts matched an
  independent file inventory and PNG-header dimension reads:

| Native interaction | Observed result |
| --- | --- |
| Apply PNG and minimum width 2000 px | 9 images; both condition summaries visible |
| Open result, move right, return with Esc | Viewer moved from 4/9 to 5/9; returned to the same filtered gallery position |
| Remove width only | 17 PNG images; extension condition retained |
| Clear remaining condition | 51 source images / 50 exact-group representatives; original gallery position restored |

The client exited normally, all 51 source SHA-256 hashes remained unchanged, and the scratch profile
was removed by its lease owner. No client overflow or exception was observed in the run logs.

## Reviewability and remaining evidence

Production / inline-test lines: screen assembly 1919 / 0, filter draft 188 / 0, dialog 292 / 0,
summary 95 / 0. The screen adds composition only; editing and validation remain separate owners.
Dedicated affected test files contain 244, 61 and 256 lines respectively.

Branch pushes alone do not trigger the CI workflow; its PR, main-push or manual-dispatch execution
remains the hosted verification owner.
Semantic indexing, text search and related-image search are subsequent R7 slices and are not covered
by this filter evidence. The stage's real-library search quality and performance acceptance remains
in R7-05.

## CI reduction — 2026-11-07

Run [42348844763](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42348844763) at `ee63a1c8`
passed Static, Windows build/integration and the five synthetic jobs. Flutter failed on an obsolete
CPU sentence; Rust Remaining failed on a fixed settings-version assertion and two unrelated domain
module-roster assertions. Its 1,228 passing Rust tests took 1,556 seconds; the Flutter wrapper started
about 240 separate commands before reaching its failure.

Removed the incidental version/text assertions and unrelated domain roster/visibility mutations.
Migration data, download authority, device switching, the availability call closure, protected DTO
loading and dangerous alternate-loader checks remain. Local focused results: one migration and
11 availability cases passed; seven Flutter cases across three suites passed in one invocation
(runner-reported test duration: three seconds).

Quick/full/extended selection and aggregate failure propagation passed their focused checks;
actionlint and PowerShell syntax checks passed. Presentation changes select two main jobs, critical
or unknown changes select nine, and nightly/extended runs select fourteen. Release workloads bind
the candidate source and no longer duplicate the scan benchmark inside the runtime verifier.
Changed large owner counts: `local_files.rs` 2,603 production / 3,351 inline-test lines; dedicated
topology tests 458 lines. No source-reading production behavior changed.

Run [42356744377](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42356744377) at `2c039388`
completed all 1,783 Flutter cases in four batches / 260 files; invocation-to-completion time was
13 minutes 27 seconds. The earlier run spent approximately 24 minutes before its stale assertion
failed. Rust Remaining passed 1,232 cases in 1,160 seconds; runner variation prevents attributing the
entire timing difference to pruning. All four Rust partitions, Windows build, scan and accessibility passed.
Static exposed a duplicate assertion tying a scan benchmark to the old verification job; `d2dd220d`
removes it while the independent release workload remains required. Its existing guardrail passed
under PowerShell 7 and Windows PowerShell 5.1. Remaining hosted results retain that run's revision.

The quick path additionally avoids unrelated native compilation/packaging checks and selects Flutter
suites through the existing Dart parser. Bounded selection probes covered direct/transitive imports,
conditional exports, parts, removed targets, cycles and runner configuration. The actual R7 filter
comparison selected 203 of 260 suites and included its filter/draft cases; that broad change includes
a shared domain type and therefore still selects full CI. No new permanent test matrix or changes to
`testing.md` were added for this refinement. Its hosted quick-path duration remains unmeasured.

## Shared-feature foundation

[ADR 0040](../architecture/0040-durable-discovery-features.md) owns common feature identity and the
independent SQLite store. Classification now reuses compatible durable features and promotes LRU
hits; clearing its LRU retains discovery features. Newly encoded features publish after source and
catalog revalidation, before category evidence. Original F32 values and existing key hashes remain
unchanged. No new model, inference worker or production retrieval dependency was added.

Rustfmt and all-feature library Clippy passed. Five real-SQLite store cases cover reopening, model
separation, budget retention, atomic rollback, corruption and foreign-database rejection. One shared
repository case covers cache promotion and LRU cleanup. The six existing analysis, ten domain and
eleven cache regressions also passed. The fixed dependency inventory check passed with updated
symbol-scoped composition registrations; no legacy exception was added.

Production / inline-test lines: store facade 193 / 0, storage admission 107 / 0, shared repository
92 / 42, common features 171 / 0, native executor 212 / 0. Dedicated store tests contain 154 lines.
Independent semantic backlog, retrieval, current-library coverage and desktop search are not yet
connected; these checks establish the persistence foundation only.

## Retrieval evaluation

An ignored Windows x64 release probe used USearch 2.26.6 with default features disabled and the
retained 368 image / 48 text-query F32 embeddings (768 dimensions). No model download or source
image read was needed. Exact F64 cosine supplied the comparison; scopes selected all candidates,
every third candidate and every seventeenth candidate before ranking.

Search expansion 128 recovered 479/480 exact top-ten results in the full scope. Expansion 512
recovered 480/480 in each scope. Full-scope median/p95 retrieval was 0.098/0.150 ms; build time was
19.2 ms. Serialized size was 1,232,948 bytes and engine-reported memory 16,783,168 bytes, excluding
process overhead. Save/load, mapped-view agreement, removal and a Unicode path passed. Native
search has no observed cancellation callback, so integration would need bounded work windows.
These small-corpus measurements do not establish full-library latency, recall or resident memory.
