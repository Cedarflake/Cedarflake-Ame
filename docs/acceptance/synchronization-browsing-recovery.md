# Synchronization browsing recovery

Date: 2026-09-09

## Scope and cause

Preview requests rejected an outdated Windows source revision before materialization. The old
source-open mapping discarded the original cause as `preview_request_superseded`, leaving repeated
pending demands without a source-reconciliation owner. Exact physical identity, size, and modified
time do not establish unchanged content when the Windows change-time revision differs.

Gallery refresh independently loaded a detail window and timeline, then retried only the detail
window. Concurrent publication could produce different revisions on every read. Folder expansion
also rejected a coherent first page merely because it was newer than the gallery; stale pagination
had no explicit replacement-window contract.

## Correction

- Initial preview source mismatches admit at most one current catalog path through a dedicated
  repository port. Shared queue capacity, durable leases, guarded discovery, and atomic catalog
  deltas own reconciliation. Existing queued work and wider recovery authorities remain unchanged.
- A typed query result reads anchor, bounded page, and timeline inside one SQLite transaction.
  Ordinary page reads and the immediate removal page do not aggregate a timeline.
- Deferred removal refresh uses the existing committed-refresh coordinator, revalidates the old
  publication after waiting, and avoids an extra read when another update already replaced it.
- Folder pages explicitly append or replace. Branch data revision and gallery invalidation revision
  are separate; scope and append-version checks remain enforced.
- Source errors retain their original code in preview failure state and diagnostic output.

No dependency, schema migration, cache format, or media mutation is introduced. A source mismatch
can add one bounded media inspection before preview regeneration. Admission backpressure and
publication conflicts preserve the existing work for later processing. Original source revision
and publication guards are unchanged.

## Verification

The canonical bridge generator completes, and all 15 asynchronous bridge contracts pass their hash,
body, and negative-fixture checks. The initial Daily invocation passes the complete lint gate,
including warnings-denied Clippy, formatting, and the pinned Dart analyzer. Its Rust suite reports
1,480 passed, four failed, and 19 explicitly ignored tests. It does not complete Daily.

Three of those failures are corrected: exact source-loading rosters now include the already-present
private catalog-identity module and the new public query-contract module, and the explicit retry
test expects the preserved source-open error. Eight source-loading contract tests, that retry test,
and three source-recovery tests pass in a serial follow-up. The new roster regression rejects 20
altered declarations without widening production source access. Earlier focused verification passes
29 preview, three atomic query/folder, and one ordinary folder-page tests.

The remaining mixed-load failure reaches its unchanged 300-second creation-to-reopen deadline.
The recorded P0 P95 is 288 ms with 25 samples; P1 completes all 2,048 entries. P2 stages 10,000
entries and completes 2,944 of its first 3,072 candidates, with 64 leased and 65 pending queue rows
at the deadline. Progress continues before failure. This is incomplete recovery evidence, not a
completed baseline or a successful full gate. Its original log is retained separately.

One isolated rerun of that exact test passes without changing the workload, P95 limit, or 300-second
deadline. It finishes in 274.12 seconds, with P0 P95 176 ms and all 10,000 staged entries, candidates,
owners, and completed owners present. The run and control are completed, authority is retired, and
the reopened baseline, checkpoint, and root are current. This does not erase the first full-run
timeout or claim a subsequently completed single Daily invocation.

An all-target follow-up compilation encounters Windows commit-memory exhaustion (OS error 1455).
After its processes exit, the focused library tests pass with one compiler job. No system memory
setting is changed.

All 71 Flutter test files pass, comprising 565 tests across the initial run and focused/remaining
reruns. The removal/update race keeps exactly two page reads; a pending deferred refresh retires
after a replacement query, and a failed refresh preserves the published page, exposes an error,
and clears loading. An inconsistent typed query result fails without automatic extra reads and
recovers on explicit retry without another scan. Final repository lint passes again, including
217 unchanged formatted files, all-target/all-feature Clippy with warnings denied, and Dart analysis
with no issues. All three broker-binary integration tests pass.

The Windows scan partition passes all three real-runner tests, including the production directory
picker and the generated atomic-query bridge. The restored detail window and timeline have matching
revisions, query identities, and result counts. The owned run exits successfully with no cleanup
failures; its isolated fixture and evidence storage are retained. The native Windows accessibility
partition passes all ten exact UIA phases, with no invalid engine tree update, no run or cleanup
failure, and confirmed primary-process exit and owned Job closure.

The unsigned Windows x64 gate passes a fresh Release application and broker build, three engine-free
window lifecycle cases, two real Debug-engine retirement cases, dependency freshness and packaged
DLL identity checks, and the catalog-free packaged bridge smoke. All 19 payload files remain
identical before and after that smoke. The gate records the pre-commit working tree explicitly as
dirty; its evidence is not rewritten to claim a build from a later commit. The runnable bundle is
`build/windows/x64/runner/Release`.

The source-recovery fixture uses the production stable location ID, edits content while preserving
size, physical identity, and modification time, and verifies new preview pixels after reconciliation.
The fixture source bytes remain unchanged by recovery. Tests use disposable fixtures. The retained
source roots are not acceptance inputs for this change.

## Responsibility and physical size

The SQLite facade decreases from 5,328 to 5,127 production lines. Its extracted query transaction
owner is 335 lines with 151 dedicated regression lines. The change-queue facade is 2,670 lines;
its only added behavior is a child-module declaration, and the admission owner is 129 lines.
The preview facade contains 612 production/test-hook lines and 1,052 inline test lines; new source
reconciliation lives in 119 production lines with 238 dedicated test lines and a narrow port.
The viewport decreases from 1,642 to 1,597 lines; query-result validation owns 36 separate lines
with 361 dedicated test lines.
The catalog mapper is 507 lines, and the folder controller is 215. The existing large owners remain
decomposition debt; new query and source-admission policy is kept behind these separate boundaries.
The local-files owner contains 4,058 production and 3,420 inline-test lines; its 12 added lines only
extend the exact test roster. The dedicated module-topology suite is 226 lines.

The immutable signed Windows release gate requires externally supplied signed application and
broker artifacts and publisher identity; a local unsigned build cannot substitute for that gate.
