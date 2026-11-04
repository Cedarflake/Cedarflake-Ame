# Continuity regressions and retained evidence

Status: binding scoped requirements under [quality-gates](../quality-gates.md).

Read this file when its workflow applies. Use the parent index to select shared workflow and
execution safeguards; the roadmap owns current delivery order and acceptance status.

The dated remediation results below retain their original revision and coverage. They are not
current-head results or a request to rerun completed stages. Preserve the affected regression
requirements; select execution through the delivery workflow and canonical roadmap.

The R2c-R SQLite protocol-read remediation adds no new public command or acceptance case. Its
production contract is exercised through the existing exact watcher-overflow case and ordinary
19-case runner. All six idempotent public catalog read families and the persistent acceptance
observer use the adapter-owned `SqliteCatalogReadExecutor`. Only rusqlite
`FileLockingProtocolFailed` is retried, on a new connection with read-only schema/identity
validation; application-owned migration and every write path remain outside the owner. Production
allows five total attempts within a 100 ms monotonic retry-admission window with 1/2/4/8 ms bounded
backoff. The window is checked only after a SQLite call completes and does not claim to interrupt an
active call. Read connections use a 100 ms SQLite busy timeout shared with that admission window;
write connections retain their five-second timeout. Busy and Locked are terminal to the protocol-
retry owner. Exhaustion must retain operation, attempts, actual elapsed time, and final structured
cause without an absolute catalog root.

The ninth-remediation focused gate was the 13-test SQLite read-retry module plus the real concurrent WAL writer/public
timeline regression. Red evidence, the eighth-remediation first failure, and the ninth-review
four-of-five reproduction remain recorded in the owning acceptance document. Closeout additionally
requires twenty consecutive fresh-nonce exact watcher-overflow passes and one complete ordinary-user
runner; a failed sample is retained and resets the conclusion rather than being retried until green.
The completed sequence spans 307-327 ms P95 and 11.463-11.872 seconds convergence; every operation
used one attempt and no protocol retry. Complete Daily, all-target/all-feature development and
Release checks, warnings-denied Clippy, and `quality_lint.ps1` pass. Because production Rust changed,
fresh unsigned x64 app/broker, PE machine, Cargokit dependency/hash, and ASCII/UTF-16 test-seam
checks are also required. These unsigned artifacts are not release candidates. Signed Release and
portable gates must continue to fail closed when the externally signed Application, broker, or exact
publisher is absent.

The tenth-remediation focused gate expands the SQLite read-retry module to 32 tests. It structurally
closes the crate-visible API to named reads, verifies raw typed protocol classification and forged-
code rejection, locks sanitized structured FRB serialization, and covers held Windows catalog
identity across delete, replacement, ABA, reparse, mismatch, success, failure, and backoff. Its
manual clock runs the exact production five-attempt/100 ms/1-2-4-8 ms policy at open, validation,
and query without using a wall-clock fixture. The migration group, current-schema no-migrate, and
256-read public WAL regression remain required. The R2c-R observer must use named typed reads for
gap, inventory, authority, location, and completed-P1 polling; high-frequency raw reopen or `expect`
is a gate failure. Canonical bridge generation, Rust/Dart hash compatibility, three PowerShell
parses, guardrail 19, ValidationOnly 19/15/4, topology/macro/source closure, no-change, tamper, root
replacement, and both disposable Cloud controls are required before the repeated production runner.
These remain non-external gates and cannot accept R2c-R or R2c-O.

The current eleventh-remediation gate is 33/33 read-retry tests. It additionally requires bundled
SQLite to report `ENABLE_SETLK_TIMEOUT`, keeps `busy_timeout = 100 ms` and `query_only` on every
read connection while the writer remains at five seconds, and proves read and write contention
timeouts separately. The exact watcher observer must combine gap, inventory, authority, and bounded
location evidence into one named typed snapshot per observation. Before aggregation, the exact
control failed at 789 typed operations; the admitted repeated sequence is 20/20 with 474-527
operations/attempts, zero protocol retries, maximum attempt one, sample-P95 P95 374 ms, maximum
sample P95 403 ms, convergence P95 12.128 seconds, and maximum convergence 12.941 seconds.

Production and closed-process acceptance waiting derive from one tracked policy byte sequence:
`tool/library_synchronization_poll_interval_ms.txt`. It must contain exactly one positive base-10
integer followed by LF; empty, zero, signs, units, extra whitespace or lines, CRLF, BOM, unsigned
overflow, and values above `9223372036854775` milliseconds fail before output changes. That exact
maximum is `floor(i64::MAX / 1000)` because Dart `Duration` stores signed 64-bit microseconds.
`./tool/quality_generate_library_synchronization_policy.ps1` deterministically writes
`library_synchronization_policy.g.dart`; `-Check` compares exact UTF-8/LF bytes without writing.
`./tool/quality_test_library_synchronization_policy.ps1` proves stale output and the invalid-input
matrix fail closed. `quality_lint.ps1` runs both the guardrail and `-Check` before formatting.

Dart production code imports the generated
`productionLibrarySynchronizationPollInterval`. `RustLibrarySynchronization.production()` takes no
arguments, while dependency injection, alternate clocks, diagnostics controls, retry schedules, and
fast cadence exist only on `@visibleForTesting .testing(...)`; analyzer configuration promotes a
production misuse of that member to an error. A Zone timer behavior test executes a successful
start, intercepts the actual `Timer.periodic` creation, and compares its `Duration` with the generated
constant. A separate behavior test proves an explicitly injected fast testing cadence remains
available. `main.dart` may call only `.production()`.

The common cfg(test) Rust `production_synchronization_cadence` module uses `include_str!` once on the
same tracked text and enforces the identical canonical form and Dart-safe maximum.
`ProductionSynchronizationCadence` is stored by `ProductionSynchronizationTestHarness`. R2c-R
composes it with readiness, crash-ready, and recovery counts without an interval parameter; worker
reports and the parent still require `1/1/0` or `1/0/1`. The value's `Duration` field is private and
neither acceptance exposes a naked interval getter. R2c-M's real helper requires the opaque value
obtained from its harness; R2c-R's counted wrapper invokes the same owned wait method before recording
the phase. A compile-time function signature rejects `Duration` at the R2c-M cadence position. The
former embedded Dart/main sources, handwritten Dart tokenizer and statement cursor, Rust cadence
`syn` visitor, semantic source scan, and mutation matrix remain removed. `syn` remains only for
independent filesystem source-topology tests.

Focused red evidence changes the tracked policy to 875 while leaving generated Dart at 250, mutates
the real periodic timer to 875, attempts `.testing()` from `main.dart`, and misrecords recovery as a
crash-ready wait. The corresponding generation check, timer assertions, fatal analyzer rule, and
Rust invocation-count assertion all fail. Restored focused green evidence includes the generator
drift/invalid matrix, 31 synchronization controller tests, three lifecycle-owner tests, two Rust
policy tests, the report-field contract, and the ordinary-user 19-case lightweight R2c-R guardrail.
Warnings-denied Clippy and the complete lint gate pass. The full ordinary-user runner passes 19/19
with closed-process P50/P95/maximum 566/703/703 ms, 24 availability probes, zero source or inventory
reads, and three bounded content opens; no-change remains exactly 100 starts, 200 polls, and 200
probes without enumeration or publication. Serial Daily passes 900 Rust tests with zero failed and
17 expected ignored, broker 3/3, all Flutter tests, and both controlled Windows integrations 2/2.
A fresh local unsigned Windows x64 Release build proves x64 PE payloads, identical packaged/Cargokit
DLL hashes, no policy source in assets or dependency manifests, and no deleted cadence-source seam
in five Rust Release artifacts. Sandbox variants stop at the expected held-parent reopen denial; the
identical ordinary-user gates pass without weakened checks. These are non-external controls and do
not accept R2c-R or R2c-O.

Phase-24 red controls prove the former numeric and consumer gaps. The generator wrote the first
Dart-unsafe `9223372036854776` value, and Rust reported that same value as an accepted invalid policy.
With tracked policy 875, R2c-M failed with actual 250 ms versus expected 875 ms. Restored controls
prove exact maximum success plus maximum-plus-one/u64-maximum/u64-overflow rejection in PowerShell
normal mode, PowerShell `-Check`, and the one Rust parser. Both the normal R2c-M harness and R2c-R
counted wrapper follow a controlled tracked 875 value before restoration to 250; their permanent
contracts also consume a synthetic exact `875\n` value through the shared object.

The phase-24 follow-up red replaces one real R2c-M wait argument with
`Duration::from_millis(250_u64)`. The former raw semantic scan and accessor-only 875 test both stay
green, demonstrating their gap. After moving the wait loop onto the opaque cadence and deleting the
naked getter, the identical mutation fails compilation with `E0308`: expected
`ProductionSynchronizationCadence`, found `Duration`. A cfg(test)-only shared sleeper capture executes
the real R2c-M helper without wall-clock delay and records exactly one 875 ms sleep request; R2c-R's
same-object wrapper records three 875 ms requests and retains `1/1/1` stage counts in its focused
contract. The capture seam is not compiled into Release.

Focused shared-policy tests, the R2c-M non-accessing guardrail, warnings-denied Clippy, and complete
ordinary-user lint pass. The complete ordinary-user R2c-R runner passes 19/19 with closed-process
P50/P95/maximum 565/578/578 ms for the initial correction and 565/585/585 ms for the follow-up opaque
API, 24 probes, zero source or inventory reads, and three bounded content opens; no-change remains
exactly 100 starts, 200 polls, and 200 probes. The sandbox denial remains
the known held-parent `C0000022`/Win32 5 boundary. Production Dart and Release payloads are unchanged,
so Daily and Release were not rebuilt for phase 24; read-only scans find no phase-24 policy/test seam
in assets or dependency files, and a fresh eight-token opaque-cadence scan finds zero matches in five
retained Release Rust artifacts. No R2c-M retained-root phase ran. These remain non-external controls
and do not accept R2c-R or R2c-O.

Each actual poll still performs one fresh O(1) availability metadata probe: caching, skipped
availability validation, source enumeration, inventory, and media reads are forbidden. The retained
red results were 78 probes in the complete runner and 75/75/77 in exact reproduction. Five corrected
exact runs each report 24 probes; the pre-binding final runner reported closed-process P95 569 ms,
and the fresh bound runner reports 24 probes with P95 581 ms. The separate no-change case continues
to require exactly 200 polls and 200 probes.

The phase-26 live-gap ownership gate adds no public script or acceptance case. Focused Rust tests
must exercise the production coordinator for native `need_rescan`, observer-ingress drop,
offline-to-available recovery, expired-lease restart, continuous journal coverage, uncovered journal
recovery, and P2 capacity rollback. A convergence test passes only when the root is `Synchronized`,
the relevant queue is empty, no automatic full scan was requested, and lane, origin, scope, intent,
lineage, and the persisted consumer all match ADR 0024. Pending journal ownership must exist before
P0 supersession. Uncovered recovery must create the P2 control, allowlisted authority, and lineage
transfer in the same transaction. Capacity failure must retain the original P0 as retryable without
a naked P1 root marker or partial P2 ownership.

Schema v30 migration coverage is mandatory for fresh catalogs, v29 upgrades, repeated current-
schema opening, conflicting destination rows, malformed partial v30 objects, and naked historical
`StartupCatchUp` P1 root gaps both with and without root publication identity. Root-generation and
publication-namespace identity are not event provenance, so every ambiguous naked row records
explicit recovery ownership and fails closed. Migration may not restore such a row to live P0 or
invent journal ranges, recovery authority, or source-media evidence. These focused checks remain
part of normal Rust and Daily verification; the unchanged ordinary-user 19-case R2c-R runner
supplies the controlled production-path regression.

The phase-27 manual-recovery gate adds no public script or R2c-R acceptance case. Focused Rust tests
must prove transactional explicit-claim begin, publish, abandon, crash/reopen recovery, pending-
journal isolation, and fail-closed reopen of malformed explicit, active foreground, and consumed
foreground states. Queue metrics and the production snapshot must report the typed block before a
manual scan, clear it only after successful publication, and restore it after abandon or crash.
Generated-bridge compatibility must remain exact; because the bridge DTO already owns
`recovery_blocked`, no regeneration is justified unless its Rust shape changes. Dart mapping and
Widget tests must preserve the field, show explicit manual-update guidance, expose a keyboard and
screen-reader operable `更新图库` action, and prove generic persistence failures do not expose that
action. Complete Daily, Release, and the 19-case runner may be deferred only to the named accumulated
closeout; a focused pass does not accept R2c-R or R2c-O.

The completed phase-27 capacity gate drives one real P0 live gap through more than
`max_attempts` promotion deferrals while P1/P2 capacity is full. Every deferral must use the typed
capacity state, refund the lease attempt, retain a bounded non-null retry deadline, and survive
restart plus lease expiry. Releasing exactly one matching slot must wake or make the gap ready within
the bounded deadline and allow exact P2 authority/claim acquisition without manual action. A ready
precise P0 event must remain separately leasable while that root gap waits, and a genuine
non-capacity failure must still exhaust at the unchanged limit. This gate does not require a schema
migration because schema v30 already persists the necessary attempt, failure-code, lease, and retry
state.

When multiple P2 authorities overlap for one root, retained inventory-source ownership must be
keyed by immutable change ID. A newer authority must not consume another run's frontier. Focused
tests must cover exact-owner resumption, independent-owner admission, pruning of an authority that
is missing, retired, or run-mismatched, catalog-validation failure before transfer, and restoration
after a deterministic worker-spawn failure.

At least one phase-26 P2 production matrix must use an explicitly published non-empty disposable
source baseline. The matrix then covers both an addition and a deletion through a real P2 consumer
and requires exact new-asset visibility, exact removed-location absence, unchanged control-source
bytes and hashes, `Synchronized`, zero queue/claim/authority residue, and no additional automatic
full-scan run. Metadata inventory and media-open counts must remain inside the ADR 0024 boundary. A
rollback-only publication mutation must demonstrate that the old empty-fixture/queue-only contract
could miss a visibility defect. Direct database fabrication is not acceptance evidence.

Accumulated closeout evidence passes the focused capacity, restart, lease-expiry, fairness,
real-error, ownership-transfer, and visibility cases plus the complete 83-test production module.
The first complete Daily run exposed a test-observation race: after valid P0-to-P2 promotion, the
helper selected the newest P2 `metadata_inventory` row instead of the retained P0
`live_notification` row. The corrected helper selects the exact P0 origin and lane; absence of that
row still times out and fails. The exact case, complete production module, and subsequent concurrent
Daily run pass.

`quality_lint.ps1` passes, the ordinary-user internal-disposable runner passes 19/19, and serial
Daily passes 927 Rust library tests with zero failed and 17 expected ignored, broker integration
3/3, all Flutter tests, and controlled Windows scan plus native accessibility 2/2 each. Generated
bridge hashes match at `941711727`. Release-profile warnings-denied Clippy passes, and a fresh local
unsigned Windows x64 build completes in 58.2 seconds. The application, packaged Rust DLL, Cargokit
Rust DLL, and independent broker are PE `0x8664` and `NotSigned`; the broker, rlib, and DLL dependency
graphs contain 83/82/82 present and current files; packaged and Cargokit DLL SHA-256 hashes match.
Seven ownership/fault test-seam tokens are absent under ASCII and UTF-16 scans across six Release
artifacts, and all ten Flutter assets contain neither the policy sources nor those test seams. This
is non-external unsigned evidence and does not accept R2c-R or R2c-O.

### Phase-31 migration-integrity gate

The phase-31 focused gate keeps schema v30 ownership migration, validation, and crash repair on one
exact contract:

- v29 running and paused foreground scans with a valid authoritative scan ID must preserve that scan
  ID, queue-row provenance, foreground owner, and publication binding through migration, then resume,
  publish, and reopen without an explicit recovery claim;
- truly naked v29 P1 root fallbacks both with and without root publication identity must still become
  conservative `explicit_recovery_required` claims, while existing conflict and malformed partial-v30
  fixtures fail closed and roll back;
- active and consumed `foreground_scan` claims must require an exact associated
  `scan_owner = 'foreground'`; wrong-owner reopen must return the typed live-gap contract error without
  terminating the scan, restoring the claim, changing the queue row, or exposing a configured path;
- current-v30 interrupted foreground cleanup and normal scan cleanup must invoke the same
  transaction-local orphan predicate. Assets owned by either handoff table remain, an asset with no
  location or handoff owner is deleted, no handoff reference dangles, cleanup failure rolls the whole
  recovery transaction back, and reopen retry is idempotent.

Run the six exact regressions first, then the complete SQLite migration and catalog modules and the
scan-library module serially. Scan tests that exercise Windows publication guards must be rerun by the
same ordinary user outside a workspace sandbox when the sandbox returns Win32 5; the denied run is not
green. The focused closeout also requires all-target/all-feature Clippy with warnings denied,
`quality_lint.ps1`, the explicit-recovery Flutter application and presentation files through
`quality_test_flutter.ps1`, exact Rust/Dart bridge-hash equality, formatting, whitespace validation,
and read-only absent-signed-bundle Release admission. It does not run Daily, the Release build, the
complete R2c-R 19-case runner, a retained root, real Cloud Files, SCM, named-pipe/FSCTL, or signing;
those remain accumulated or external closeout boundaries.

### Phase-32 leased capacity-gap and reserved-code gate

The phase-32 queue gate requires a due exact typed capacity-deferred P0 root gap to be exercised in
both unleased and leased states while precise live path work arrives. The gap must retain its lease,
ownership, retry state, and P0 admission protection; the precise path must become independently
leasable and visible before P2 capacity is available. Duplicate precise evidence must coalesce, and
ordinary P0 rows must retain the existing coalescing and fairness rules. The production control must
hold P2 capacity full, pause the real live worker after leasing the gap, publish a real created image
through the independent path row, and prove no automatic full scan occurred.

The failure-code gate treats `live_gap_p2_capacity_` as a reserved namespace. Generic retry must
reject both the current code and a future namespace member with a path-free typed error and an
unchanged lease/queue transaction state. Lease-expiry refund, maximum-attempt exemption,
metrics, and wake-up require the exact typed lane, origin, intent, root/path shape, status, code, and
absent recovery claim. Wrong lane, scope, status, or claim state must not receive an exemption, while
a genuine non-capacity failure must still become terminal at the configured limit. These checks use
schema v30; a new migration is not part of this gate.

Run the exact red/green controls, phase-27 retry-budget and phase-26 visibility tests, phase-29
retained-owner tests, then the complete queue, production, persistent-journal, migration, and scan
groups serially. Closeout additionally requires all-target/all-feature development and Release
Clippy with warnings denied, the 56 focused Flutter application/presentation tests through
`quality_test_flutter.ps1`, exact bridge-hash equality, `quality_lint.ps1`, the ordinary-user 19-case
runner, canonical Daily, a fresh unsigned Windows x64 Release, PE machine and `NotSigned` checks,
dependency freshness, packaged/Cargokit DLL hash equality, Release ASCII/UTF-16LE test-seam scans,
Flutter asset scans, formatting, and tracked-diff whitespace validation. A sandbox denial or a
failed first run is recorded and is not green; any rerun uses the identical ordinary-user command.

The accumulated phase-32 evidence passes 82 queue, 84 production, 42 persistent-journal, 65
migration, and 36 runnable scan tests with two expected ignores; focused Flutter passes 56/56,
bridge hash is `941711727`, and the internal-disposable runner passes 19/19. The final canonical
Daily passes 940 runnable Rust tests with 17 expected ignores, broker integration 3/3, all Flutter
tests, and both controlled Windows integrations 2/2. The fresh unsigned Release builds in 119.75
seconds; four PE images are `0x8664` and `NotSigned`, dependency graphs are current at 83/82/82,
packaged and Cargokit DLL hashes match, six Release artifacts contain none of twenty executable
test-seam strings, and ten Flutter assets contain none of twenty-four policy/test tokens.

This is non-external evidence only. Retained roots, real Cloud Files, SCM/service lifecycle, real
named-pipe/FSCTL journal access, signing, and the final independent full-range audit remain outside
this gate. R2c-R remains not accepted and R2c-O remains active.

### Worker-report preparation boundary

On 2026-10-12, hosted run `38052664349` on `9a47c70` is confirmed to stop in the exact report
guardrail's five-minute Cargo child, before the main Rust suite. That invocation includes compilation;
its discarded child output cannot distinguish a cold build from test execution. The correction
separates bounded preparation and execution, without attributing the historical timeout to an
unobserved phase or changing the exact test's deadline.

The shared harness facade shrinks from 3137 to 3102 lines and the dedicated guardrail from 2107 to
2037. The extracted Cargo admission and report-lifecycle owners are 78 and 110 lines, with no inline
tests; the new dedicated boundary regression is 204 lines. No Rust/Dart product or inline-test code
changes. Command admission, held-source closure, independent cleanup, the new source-count boundary
and the full executable 19-case guardrail pass locally. Standalone lint passes. Independent review's
single output-bound finding is corrected by counting the truncation suffix within 512 characters;
the strengthened per-line regression and scoped recheck pass. The serial Daily's Rust main suite
passes 1691 cases with 23 existing ignores, plus three supplemental cases. Subsequent whole-Daily
and hosted results retain their own completion requirements; these focused results do not close
C01, remaining native variants or R2c acceptance.
