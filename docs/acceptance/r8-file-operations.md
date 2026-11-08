# R8 filesystem verification

Status: planned verification procedure; no R8 execution or recovery pass is recorded here.

The [roadmap](../roadmap.md) owns activation and delivery status. The
[merged product scope](../product/organization-workflows.md#r8---file-organization-and-recovery)
owns authorization and file-operation behavior. Windows Sandbox is not a prerequisite.

## Disposable data and execution boundaries

Local client checks use copies of the established `build/test` sample corpus in a leased
`Invoke-AmeAcceptanceScratch` directory. Keep source fixtures, targets, catalog, and journal in
separate owned subdirectories. Follow the existing
[storage and cleanup procedure](quality/execution-safety.md#execution-scope-and-storage); preserve
the reusable corpus and original catalogs. Never give the test process real-library roots or a
production profile. No real photographs or catalogs enter hosted CI.

Scratch storage supplies ownership and cleanup, not operating-system isolation. The production
operation boundary must validate resolved roots, file identity and target state, and reject path
escape or unapproved reparse traversal. First exercise boundary attacks and process termination in
a disposable hosted Windows job, using synthetic files and fixture-only sentinel targets. Local
client verification follows those checks. Never point a deliberately hostile fixture at user data.

Record relative paths, file sizes and content hashes before and after a run. Include untouched
sibling files outside the operation's allowed source and target roots but inside owned test storage.
Assert the exact permitted changes and retain compact journal/catalog evidence; success messages
alone do not establish integrity. Join owned processes before cleanup. Link fixtures must remove
their owned links without traversal before returning to scratch cleanup, which rejects reparse points.

## Required outcomes

Use the real Windows operation adapter and persisted catalog/journal for execution checks. The
[execution plan](../plans/r8-file-organization.md#windows-capability-admission) owns API admission.
Fault injection supplies a reproducible interruption or error; assertions inspect persisted results
after recovery. Test Ame's parameters, authority, result mapping and recovery rather than duplicating
the system's internal copying algorithm.

| Scenario | Required observation |
| --- | --- |
| Preflight, rejected or stale authorization | Source and target paths and bytes stay unchanged; a changed selection cannot execute an earlier request |
| Copy, same-volume move or rename | Only authorized paths change; content is preserved; copy keeps its source; catalog identity and durable album/favorite decisions remain correct |
| Collision, locked file, unavailable root, invalid name, Chinese or long path | Supported paths work; rejected items retain their sources and existing targets; partial results identify the affected items |
| Root replacement, path escape, reparse point or changed target | Execution stops before an unauthorized write; out-of-scope sentinel files stay unchanged |
| Cross-volume move | A complete verified target is published before authorized source removal; a failed or interrupted copy retains the source |
| Cancellation, database failure, process termination | Restart reconciles disk, journal and catalog without losing the sole valid copy, repeating a completed mutation, or reporting unfinished work as complete |
| Recycle and supported system restoration | Only the run's fixture items are affected; non-recyclable items fail before mutation; unrelated recycle-bin contents and files are untouched |
| Native completion and progress callbacks | Overall success with aborted or failed items is not reported as full success; actual renamed destinations reach the catalog; estimates and stopping states remain accurate |
| Cloud-only or changed source | No implicit hydration; stale or unavailable evidence cannot authorize mutation |

Inject failures at Ame-owned boundaries: durable intent, native submission, receipt of an item
result, and catalog reconciliation. In native integration, also interrupt real transfer and exercise
disk-full, collision, lock and cancellation behavior. Do not fabricate callbacks for undocumented
Windows internal stages. A missing completion callback must leave a recoverable uncertain outcome.
Check both sides of a durable transition only where recovery differs; avoid equivalent combinations.
Recovery preserves the original authorization scope and rechecks files before further mutation.

## Windows integration and workload selection

Small integrity and recovery fixtures run for changes affecting file operations. Desktop checks
cover choosing an action, its target, conflict resolution, progress, cancellation and recovery with
sample copies; no preview UI is part of acceptance. Existing unrelated UI changes do not acquire a
new full filesystem suite. Follow the [test-design standard](../development/testing.md) and
[existing CI selection](quality/gate-selection.md).

Hosted Windows integration verifies actual file locks, volume identity, supported recycle behavior
and native publication semantics. Cross-volume acceptance requires two genuinely distinct volumes
in that disposable environment; two directories or an injected cross-volume error do not prove it.
Prepare a bounded disposable volume fixture there when implementing this workflow, with cleanup
scoped to its created resources. Do not repartition or configure the workstation for these checks.

Permission and disk-full errors can be injected for focused recovery tests; native guarantees still
need their applicable Windows integration evidence. Recycle-only safety includes a target that
cannot recycle and must remain intact; observing permanent deletion after a callback is a failure,
not a successful rejection. Test restoration through the admitted system route and restart Ame to
verify its independent history, without relying on Explorer's undo stack. A missing runner capability
needs a scoped equivalent Windows check or remains an explicit gap for that action. Small fixtures
establish correctness; large batches and sustained resource checks belong to affected performance
acceptance or release qualification.

The existing scratch helper can be reused. The R8 executor, fault-injection points, operation tests
and cross-volume fixture still need implementation; this procedure records their intended checks.
