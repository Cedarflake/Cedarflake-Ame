# R2c runtime lifecycle review

Date: 2026-09-06

Status: implementation audit recorded. Full-revision delivery gates are owned by the attached
PR checks; authorization-bound external acceptance remains separate.

## Scope and safety

The review covers the accumulated PR #12 runtime changes against the accepted Windows 11 x64
contract: first import, explicit update, pause/cancel/exit, manual continuation, root removal,
catalog and preview cleanup, preview/source identity, and their actual controls. It also reviews
responsibility boundaries, physical file size, naming, and formatting. Source inspection and
negative interleavings complement tests; passing an existing suite is not proof of an unexamined
lifecycle.

Execution uses disposable generated media and isolated catalogs. It does not modify retained source
libraries, hydrate cloud placeholders, install services, sign, merge, or publish a release. An old
green PR head does not validate subsequent working-tree changes.

## Findings and owning repairs

| Boundary | Failure traced | Owning repair and required evidence |
| --- | --- | --- |
| First import | Persisted unfinished work could be mistaken for execution or a completed baseline | Revocable process execution lease and typed synchronization admission; cancelled work stays terminal, interrupted first import waits for explicit continuation |
| Continuation | A previously visited directory could change while a partial import was stopped | Explicit continuation rebuilds untrusted unpublished inventory while preserving compatible derived previews; offline changes to an already visited directory are tested |
| Terminal scan cleanup | A late abandon call could delete published locations even when no live scan state was retired | Only the transaction obtaining the live-to-terminal transition owns cleanup; active and historical published scans, duplicate cancellation, and missing scans retain their durable owners |
| Retained task | Paused intent reserved gallery interaction; active cancellation could not durably cancel a retained checkpoint | Independent immutable primary task, read-only restoration, asynchronous exact-ID cancellation, and commit-aware removal |
| Task and query UI | Other-root updates, query failure, or refresh could hide or overwrite actionable primary work | Separate query activity and task-surface selection; committed publication retries only display reload, with real control-path regressions |
| Committed multi-root refresh | The native two-root update reached committed catalogs but one task reported display-refresh failure | Preserve the native terminal assertion and distinguish query supersession from genuine catalog failure at the shared refresh owner; do not retry source scans or treat every superseded read as success |
| Reclamation | Request admission and worker retirement could strand pending work or revive terminal state | One operation registry owns request generation, pending handoff, attempt preemption, cancellation, and terminal publication |
| Preview recovery | An old missing-file or size observation could downgrade a same-key regeneration | Shared final health observation and conditional SQL under publication exclusion; zero-wait database admission and deferred work release the guard |
| Preview relocation | Settings could consume pending ownership before all catalog references were reset | Target initialization, idempotent full catalog reset, then settings retirement; interrupted commits and source-safe fallback are tested |
| Catalog relocation | Saving a new empty-catalog location, then importing before restart, stranded the import in the old catalog | Configuration persistence and root registration share bounded admission; a pending catalog switch rejects new import until restart |
| Original viewing | Path-based `FileImage` bypassed source availability, identity, and namespace guards | Opaque native source lease plus bounded latest-intent reader; approved-path buffer copy completes before exact lease release |
| Retired viewer | An image completer could discard its error handler before a late cancelled read or frame completed | A narrow stream resource owner rejects actually retired errors and releases late codecs/frames; regressions retire the view across frames before completing acquisition, buffer copy, or first-frame decoding |
| Native verification | Original decoding plus free slots after closing did not prove guarded decoding | Occupy both real native slots, require UI source failure, release, click Retry, decode the original, navigate, close, and reacquire both slots |
| Native run lifecycle | The scan runner lacked a parent deadline and could lose original failure evidence during cleanup | Reuse owned-process execution; preserve current logs and the original failure separately from cleanup, with fresh isolated storage and no unproven recursive deletion |
| Mixed-load fixture lifecycle | An assertion before explicit teardown could detach the competing writer and contaminate later cases | One test-owned fixture releases the gate, drains the exact runtime under its original deadline, and joins the writer on success or unwind; one-shot drain panic retains cleanup responsibility |
| Bridge quality gate | An asynchronous-method check could borrow a following method's Normal call | Extract exact uniquely named method bodies after excluding literals/comments; negative fixtures prove Sync, missing, duplicate, and incorrect-owner methods are rejected |

The preview health and relocation changes add no schema version. Durable dimensions and source
configuration are not treated as cache. Existing first-import lifecycle and source-generation
migrations retain their separate regression requirements.

## Actual interaction evidence

The controlled native scan workflow uses the production Rust bridge, catalog, and synchronization
adapter. It exercises the real directory picker, a Chinese nested path with a wrong-extension PNG,
a corrupt image isolated from the successful scan, preview regeneration, and two configured roots.
Its helpers exercise retained import navigation and durable Cancel, multi-root Update through the
selection dialog, Remove through confirmation, and Settings/gallery round trips. The retained
fixture compares every one of its 1,024 tiny source files after cancellation and removal; the
two-root fixture compares every source file and directory entry count.

These workflows establish actual control-to-backend behavior and terminal/source invariants. They
do not establish 80,000-item removal latency, actual process-crash recovery, every cloud provider,
or performance on a retained personal library. The viewer's Cloud tests cover official tag/state
classification and ordinary native namespace protection, not a live Cloud Files provider.

## Physical architecture review

Counts include blank lines. Where a file has a top-level inline test module, production prefix and
test tail are reported separately; small `cfg(test)` hooks can remain in the prefix. Generated
bridge files are not compared with handwritten owners.

The primary Dart controller decreased from 926 to 353 lines. Its task lifecycle, active run,
checkpoint restoration, immutable task state, and query state have separate cohesive owners.
Reclamation decreased from a 1,062-line application file to a 468-line facade plus its operation
registry and dedicated tests. Storage configuration, catalog-transition admission, preview-root
activation, and their tests no longer extend one combined owner.

Large historical owners remain genuine physical debt, not a completed cleanup:

| Owner | Reviewed size | Meaningful remaining boundary |
| --- | ---: | --- |
| Synchronization production | 14,651 total; 4,150 prefix and 10,501 inline-test tail | Runtime registry and Live/Journal/Recovery state machines, with each scenario suite |
| Catalog migrations | 15,556 total; 10,293 prefix and 5,263 inline-test tail | Shared historical validators and exact shrink-only repair, preserving centralized migration order |
| SQLite catalog facade | 5,545 | Persistence operations continue to move with their typed invariants, not forwarding fragments |
| Local filesystem adapter | 7,662 total; 4,283 prefix and 3,379 inline-test tail | Namespace, metadata discovery, and source-read adapters with their contract suites |
| Persistent journal adapter | 8,443 total; 3,734 prefix and 4,709 inline-test tail | Checkpoint, baseline, lineage, and capability responsibilities |
| Dart viewport | 1,605 | Query transition and retained-window navigation |
| Unified gallery screen | 2,428 | Synchronization notification and display-refresh coordination |
| Gallery wall | 2,186 | Revisit when independent layout/retention lifecycles require a boundary |
| Scan command facade | 1,173 | Remaining command sequencing, distinct from extracted execution and cancellation ownership |
| SQLite change-queue tests | 6,999 dedicated-test lines | Separate coherent queue scenarios with their owning production boundaries |

The new source-reader Rust application/API/guard owners are 226/40/190 lines, with 453/80 lines of
dedicated tests. Dart reader/scheduler/adapter/provider/stream/viewer owners are
21/118/61/138/106/162 lines. Their four dedicated suites are 188/277/231/223 lines. The bridge gate
separates the Daily facade, source parsing, exact contract policy, read-only command, and negative
fixtures instead of expanding the workflow's regex block. These measurements do not
justify moving a monolith unchanged or imposing a universal line cap. Follow-up order remains in
the single [canonical roadmap](../roadmap.md), with architecture ownership in
[ADR 0025](../architecture/0025-invariant-owned-workflow-modules.md).

Query-refresh admission is a 130-line production owner with 134-line coordinator and 396-line
application regressions. The Windows scan runner has a 16-line public facade, a 205-line execution
owner reusing existing process-tree supervision, and a 200-line lifecycle guardrail suite.
Terminal scan cleanup has a separate 213-line production owner and a 403-line regression suite.
The mixed-load priority scenario and its exception-safe fixture moved out of the production
coordinator's inline test tail. This is a scenario boundary, not completion of the larger runtime
split; its 913-line dedicated suite preserves the original workload, page counts, P95 assertion,
and progress deadline.
The primary-scan lifecycle and multi-root update owners remain 587 and 769 lines. Dedicated test
debt is also explicit: the SQLite catalog suite is 8,064 lines, scan suite 5,454,
Dart controller suite 3,625, unified-screen suite 3,977, and viewer-position suite 2,028. Adding a
small production module does not make these large test owners physically small or complete their
scenario-based separation.

## Verification ledger

- Earlier focused first-import, retained cancellation, reclamation, controller/query/task, and
  tooltip regressions passed during implementation; subsequent source-reader and storage changes
  require fresh complete verification.
- Current Release Rust compilation, seven preview-health regressions, 25 storage lifecycle tests,
  ten source-reader native/contract tests, and all-target/all-feature Clippy passed. Dart analysis
  passed before the final bounded test-pump adjustment. Bridge hash `-1659252675` matches across
  generated Rust and Dart; 14 exact asynchronous contracts and 25 rejection fixtures passed.
- The four current source-reader/stream/viewer suites passed all 21 tests. The first fresh Windows
  integration built successfully and passed retained-import cancellation and real-picker cancellation;
  its third scenario passed guarded original retry/navigation but exposed a multi-root display-refresh
  failure before root-removal verification. That run is failed evidence, not native workflow acceptance.
- The corrected query-refresh owner passes ten focused coordinator/application regressions and
  fatal-warning/fatal-info Dart analysis. Eleven related Flutter suites pass 194 tests covering
  retained tasks, cancellation, query failures, menu semantics, direct jumps, previews, and viewer
  navigation. The repeated controlled Windows scan workflow passes all three scenarios, including
  multi-root committed refresh and root removal, with unchanged fixture source bytes. Its new parent
  runner records exit zero and no cleanup failures in retained GUID-isolated evidence.
- The current Daily Windows-accessibility partition passes both scenarios, all ten exact native
  activation/UIA phases, and the engine-stderr rejection check. The scan and accessibility passes
  use current production binaries; neither claims retained-library performance or Cloud-provider
  acceptance.
- A complete serial Rust run reported 1,202 passed, three failed, and 17 explicitly ignored tests.
  Two failures exposed old fixtures that treated an unpublished root as a replacement-scan baseline;
  the other reached the unchanged mixed-load post-sample progress deadline. These are retained failed
  results, not erased by focused passes. Fixture repairs preserve complete publication/claim checks;
  additional raw-spool diagnostics distinguish actual source progress from a zero staged-page count.
- The repaired baseline fixtures pass the original queue-retirement and explicit-claim restoration
  assertions. All five terminal-scan cleanup regressions pass, including active and historical
  published owners, duplicate cancellation, and missing scans across catalog reopen.
- The extracted mixed-load suite passes six fixture-lifecycle regressions and the original pressure
  scenario: 25 samples, 2,048 P1 changes, 10,000 P2 source files, 4,095 staged entries, and competing
  catalog writes. Event-to-visible P95 is 326 ms with no sample above one second. Both background
  lanes complete under the unchanged post-sample deadline. This focused pass does not establish a
  cause for the earlier full-suite timeout or replace the full-revision Rust CI partition.
- Production-only Rust compilation exposed a test-gated admission method that unit-test builds
  would not expose. The existing nonwaiting admission is now compiled for production.
- Native source review exposed a directory-open NoRecall flag incompatible with the accepted
  Windows directory contract. Directory metadata guards and final media reads now use their
  distinct supported flags; the real nested-path read regression remains mandatory.
- Native execution corrected two invalid test assumptions: official Cloud-family tags also encode
  partial states, and Windows share modes do not exclude attribute-only handles. Tests distinguish
  partial from available directory tags and check data-write/delete/rename exclusion without
  claiming immutable metadata. No production availability check was weakened.
- The complete PowerShell quality guardrails passed; the following format-only gate rejected the
  newly added query-refresh files while implementation was still in progress. That command did not
  complete the full lint gate and did not modify those files.
- After formatting the owned additions, the complete local lint gate passes: PowerShell syntax and
  positive/negative guardrails, non-mutating generation and format checks, all-target/all-feature
  Clippy with warnings denied, and Dart analysis with informational and warning lints fatal.
- Independent source reviews cover the final query-refresh, terminal-scan, original-reader, storage,
  pressure-fixture, and native-runner ownership changes. The historical physical debt above remains
  explicit; no review claims the absence of every possible defect.
- The delivered revision must pass every ordinary [PR #12 quality check](https://github.com/Cedarflake/Cedarflake-Ame/pull/12/checks):
  all Daily partitions, generated bridge compatibility, credential-free unsigned Release, and all
  three hosted synthetic workloads. These immutable check results own full-revision verification,
  not this local ledger or an earlier green head. Protected signing and authorization-bound
  real-library acceptance are not substituted with credential-free CI results.
