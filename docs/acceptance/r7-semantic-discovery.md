# R7 implementation evidence

Date: 2026-11-07
Scope: filters, shared features, semantic retrieval and resource-aware scheduling on
`codex/r7-semantic-discovery`.
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
| Draft editing | `AlertDialog` and `ChoiceChip` expose common format/range options. `Form` and `TextFormField` appear for custom values; `validateGranularly` identifies an invalid field to reveal through framework scrolling. |
| Date field and size unit | `SegmentedButton` supplies mutually exclusive typed choices in the installed SDK. |
| Calendar | `InputChip` opens `showDatePicker`, a compact desktop dialog. Two independent bounds support an open-ended interval and individual clearing. |
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

Original filter-slice production / inline-test lines: screen assembly 1919 / 0, filter draft 188 / 0,
dialog 292 / 0, summary 95 / 0. Dedicated affected tests contained 244, 61 and 256 lines respectively.
Current connected-slice counts appear below; editing and validation remain separate owners.

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
These checks establish the persistence foundation. The connected search and scheduling checks below
retain their separate scope.

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

## Parallel-worker evaluation — 2026-11-07

The local-primary comparison used the production Profile worker, protocol 2, the unchanged admitted
SigLIP 2 artifacts, ORT CPU 1.31.1 and DirectML 1.24.4 / 1.15.4. Worker SHA-256:
`3f06854bd9d5e218f22b88256a036691e09eec5cc879178532f4edd9f6ee3f63`.
The host has an i7-13620H, 10 physical / 16 logical processors, 15.80 GiB usable RAM and an RTX 4050
Laptop GPU. CPU-only results below disable GPU inference on this host; they do not characterize a
low-core or low-memory machine.

GPU runs used all 368 retained previews and 48 bilingual descriptions. CPU-only runs used 64 evenly
spaced previews from that corpus and the same 48 descriptions. The harness prepared bounded image
tensors and drove the actual native protocol, with one below-normal-priority worker per encoder,
one inference thread per worker and the existing one-process / 2-GiB Windows Job limits. It opened
no originals or catalogs. The application scheduler, decoder and semantic retrieval were outside
this experiment. Each paired configuration ran sequential, parallel, parallel, sequential batches;
sequential batches retained both workers to distinguish concurrent execution from model reloads.

| Paired configuration | Sequential image median | Parallel image median | Sequential text median | Parallel text median | Peak combined private commit |
| --- | --- | --- | --- | --- | --- |
| GPU image + CPU text | 13.07 ms | 13.11 ms | 63.50 ms | 52.53 ms | 1,375 MiB |
| CPU image + CPU text | 363.37 ms | 368.80 ms | 37.40 ms | 42.60 ms | 1,644 MiB |

Medians pool both runs of each mode. GPU-run text timing varied substantially: per-run medians were
36–136 ms sequential and 38–150 ms parallel, so the pooled difference does not establish faster text
encoding. The two GPU image batches took 5.27 / 4.81 seconds sequential and 5.28 / 5.06 parallel.
CPU image batches took 23.42 / 23.31 seconds sequential and 25.72 / 23.33 parallel. Every produced
embedding matched its retained same-backend reference exactly, including concurrent execution.

The isolated image workers used approximately 1,006 MiB private commit on GPU and 1,277 MiB on CPU;
the additional text worker used approximately 368 MiB during steady execution. A separate cold-load
measurement observed a 702-MiB text private-commit peak and a 1,339-MiB GPU-image peak. Their sum is an
upper bound from individual process peaks, not a measured simultaneous peak. Working-set trimming
occurred during the runs; private commit and physical resident memory are distinct measurements.

Starting text while the GPU image worker continued required 1.79 seconds through the first query.
Serial image retirement plus text startup/query took 1.62–1.71 seconds; retiring text and recreating
the GPU worker through its first image took another 1.70–1.86 seconds. Parallelism therefore avoids
background interruption and image-worker replacement, while first text use retains cold-start cost.
Terminating the owned text worker during a separate 64-image GPU pass left all image outputs valid;
this establishes process isolation, not application cancellation/restart acceptance.

The measured benefit of parallel workers is background continuity and avoided image-worker reloads,
rather than faster individual inference. [ADR 0041](../architecture/0041-resource-aware-analysis-scheduling.md)
admits bounded concurrency when device resources permit and serial execution otherwise. These process
measurements do not yet verify that scheduler's foreground latency or mode transitions in Ame.
All owned processes retired, leased scratch directories were removed and
all 368 retained preview hashes remained unchanged. Raw timings and exploratory harnesses remain
under ignored `build/r7-semantic-eval`; no new permanent CI workload was added.

## Connected scheduling and search

The application now owns independent foreground search and background indexing behind the same
two-encoder scheduler. Cancellation remains request-specific. Low capacity uses serial admission;
memory pressure closes parallel admission, with delayed recovery. Cached text queries do not extend
the serial model-retention interval. Source-image preparation retains its file lease through final
reference revalidation. Retrieval reads a fixed catalog snapshot and feature generation in bounded
windows, so background publication cannot invalidate or reorder an active result manifest.

Eleven focused Rust cases passed across scheduler concurrency/retirement, capacity hysteresis and
dispatch cancellation. A real SQLite two-connection case additionally passed: a background catalog
publication commits while the reader finishes its original 257-item snapshot; a fresh reader sees
the replacement. All-feature library Clippy and the fixed dependency inventory passed. Dart analysis,
two search-coordinator cases, three retained search-focus cases and six search-availability cases
passed. The coordinator cases cover stale replies/errors, IME composition, Enter submission and
scope/mode continuity; they do not establish native desktop acceptance.

The native text worker also completed 48 bilingual descriptions and the maximum 512-character input
under the new 1-GiB Windows Job limit while a GPU worker processed 32 retained previews. Text private
commit peaked at 702.2 MiB, with a 33.5-ms median warm query. Both encoders matched their retained
reference vectors exactly. Both processes retired and all 368 retained preview hashes were unchanged.
This run validates the worker limit; connected desktop scheduling is verified separately.

An isolated optimized native desktop run indexed all 368 retained previews while servicing searches.
Repeated cached queries took 10–41 ms as coverage advanced; the first query, including model startup,
took 4.54 seconds. The connected 48-query bilingual run retained the earlier fixed-target results:
Chinese top-one/top-five 15/20 of 24, English 19/23 of 24. Median query latency was 72.40 ms and p95
105.18 ms; one cold text-worker restart took 2.41 seconds. A root-restricted query counted 128 eligible
images, and reference-image retrieval excluded its source and returned the other 367 images.
These measurements describe the retained preview corpus, not whole-library latency. This initial
run ranked all eligible candidates; the subsequent relevance admission below changes returned counts.

The process sampler observed at most two model workers, peaking at 1,603 MiB combined private commit.
Idle workers retired, normal window close exited successfully, all owned workers joined, disposable
storage was removed and all 368 source hashes remained unchanged. Native search input and relevance
results were exercised in the window. This pass exposed inherited gallery positioning and rejected
position seeks on relevance results; the affected regression now verifies ordered pages and seeks
for capture/name/modification settings against a fixed shuffled SQLite fixture.

The rebuilt desktop verified the revised Material format/range chips and date picker. Applying PNG
returned zero results in the all-JPEG corpus; removing that condition restored 368. The header showed
only the count and the search bar omitted trailing scope text. A new `fox` query started at position
0%; dragging to 56% loaded the selected result window and dragging back restored the first result.
Opening that image, moving to the next and returning retained relevance order and the gallery
position. `查找相关图片` switched to 367 results at position 0%, excluding its reference. Normal close
returned exit code zero, all owned workers retired, scratch storage was removed and all 368 preview
hashes remained unchanged. This pass does not cover restart recovery or large-library latency.

### Relevance admission

The initial connected search ranked every indexed candidate, so small libraries returned the entire
library. The relevance rule in [ADR 0040](../architecture/0040-durable-discovery-features.md#relevance-admission)
removes the weak tail and permits an empty result. Reusing the frozen F32 vectors, it retains the
fixed target for 21/24 Chinese and 24/24 English descriptions. Their existing top-five target counts
remain 20/24 and 23/24. Median returned counts are 5.5 and 1; these fixed-target measurements do not
label every returned image as relevant.

Six short-term probes and ten additional descriptions used the same admitted native text worker.
`fox` selects two of 368 images; the Chinese equivalent selects ten. Eight of the ten additional
descriptions produce no matches, including both language versions of a red helicopter in snow,
microscope bacteria and a coral reef with a diver. Chinese rocket-launch and circuit-board queries
still return candidates. The heuristic reduces irrelevant tails but retains the model's semantic
errors; no probability or universal accuracy claim follows from these cosine cutoffs.

The rebuilt native client returned only `B142.jpg` and `B143.jpg` for `fox`. The complete description
`a red helicopter parked in snow` returned zero with `没有找到匹配的图片`; clearing restored all 368
images. Normal close succeeded and all retained preview hashes were unchanged. Two focused policy
regressions, changed-file Dart analysis and all-feature library Clippy passed.

Production / inline-test lines: scheduler 184 / 0, admission 245 / 0, search execution 119 / 0,
index pass 174 / 0, retrieval 154 / 0, image preparation 126 / 0, presentation coordinator 338 / 0,
screen assembly 1977 / 0, filter dialog 316 / 0 and range choices 125 / 0. Dedicated scheduler tests
contain 214 lines, coordinator tests 164 lines, relevance tests 46 lines and filter interaction tests
267 lines. New search
widgets reuse the existing Material search bar, menus, notification controller, gallery and viewer.
