# Acceptance evidence index

This directory owns verification requirements and dated evidence. The [roadmap](../roadmap.md)
alone owns stage order and current acceptance status. An earlier pass, an implemented test, or a
recorded local checkpoint does not establish that the current product source passes its full gate.

## Current entrypoints

| Record | Responsibility |
| --- | --- |
| [Quality gates](quality-gates.md) | Canonical commands, workload admission, hosted/client boundaries, and authorization requirements |
| [Library continuity contract](library-continuity.md) | R2c safety, ownership, queue, reconciliation, failure, lifecycle, and acceptance requirements |
| [Interleaving remediation](r2c-interleaving-remediation.md) | Current accumulated runtime findings, bounded retirement, mixed-load failures, native evidence, and source-specific results |
| [Browsing recovery](synchronization-browsing-recovery.md) | The 2026-09-09 source-version, query-snapshot, and folder recovery correction and its verification limits |
| [LiveOnly subtree recovery](r2c-live-gap-recovery.md) | C04 causal repair, retained/fresh bulk client evidence, source/resource limits and unresolved C05 browsing observations |
| [Browsing diagnosis](r2c-browsing-diagnosis.md) | C05 controlled navigation results, confirmed presentation defects, unresolved native blank/gray cause and bounded stop |
| [Library count reconciliation](library-count-reconciliation.md) | Same-directory Photos/Ame totals, metadata path census, admission differences and unresolved historical revision evidence |
| [Runtime lifecycle audit](r2c-runtime-lifecycle-audit.md) | Cross-workflow findings and ownership/physical-size evidence |
| [Media and publication audit](r2c-media-and-publication-audit.md) | Real media encodings, publication boundaries, and related audit evidence |
| [Gallery foundation](gallery-foundation.md) | Historical R0/R1/R2a/R2b acceptance, frozen interaction comparison, and retained preview/geometry requirements |
| [Read-only real-library acceptance](read-only-real-library.md) | Explicitly authorized catalog/source evidence; no standing authority for another run |

The [R2c execution plan](../plans/r2c-closeout.md) specifies how to investigate and repair the current
bounded batch. Add observations and results here, not another priority queue in that plan or this index.
Keep private catalogs/media, machine-specific roots, account labels, and raw user logs out of records.

## Stage records and historical applicability

| Scope | Evidence | Interpretation |
| --- | --- | --- |
| R2c-A–D | [Contracts](r2c-a-blue-team.md), [watcher](r2c-b-windows-observation.md), [queue](r2c-c-durable-change-queue.md), [atomic deltas](r2c-d-incremental-delta-publication.md) | Dated foundations; current callers must preserve accepted invariants |
| R2c-E–F | [UI lifecycle](r2c-e-production-ui-lifecycle.md), [recovery](r2c-f-recovery-consistency.md) | Later ADRs supersede historical recovery/scan policy where stated |
| R2c-G–H | [Direct USN](r2c-g-usn-downtime-catch-up.md), [large-library reliability](r2c-h-large-library-reliability.md) | Historical direct-desktop model; not ADR 0024 acceptance |
| R2c-I–M | [Cutover](r2c-i-non-usn-cutover.md), [inventory](r2c-j-metadata-inventory.md), [paging](r2c-k-pageable-continuity.md), [lifecycle](r2c-l-lifecycle-presentation.md), [replacement](r2c-m-replacement-reliability.md) | Superseded ADR 0023 model; retained source-safety/history only where applicable |
| R2c-N–O | [ADR 0024](../architecture/0024-windows-change-driven-library-continuity.md) and [quality-gate boundaries](quality-gates.md) | Protocol, broker, installer and external gates; no invented completed acceptance record |
| R2c-P | [Persistent journal](r2c-p-persistent-journal-continuity.md) | Implementation evidence distinct from external acceptance |
| R2c-Q | [Priority runtime](r2c-q-priority-runtime.md) | Implementation, candidate ownership, baseline and lifecycle evidence |
| R2c-R | [Change-driven reliability](r2c-r-change-driven-reliability.md) | Controlled Windows 11 evidence; external and final accumulated acceptance remain separate |

## Historical roadmap provenance

The 2026-09-09 organization removed duplicated implementation chronologies and drifting status
snapshots from the active roadmap. The exact original remains in Git at
`643f7b8f71971ce33416cc87aa94bb84f41d4e05:docs/roadmap.md`:

```powershell
git show 643f7b8f71971ce33416cc87aa94bb84f41d4e05:docs/roadmap.md
```

| Former roadmap content | Current owner or retrieval path |
| --- | --- |
| Sections 1–3 and 8–9: product, concepts, reference policy, exclusions | [Workbench contract](../product/workbench.md); historical reference observations in [foundation evidence](gallery-foundation.md) |
| Section 4: accepted UI detail | [Gallery UI contract](../product/gallery-ui.md) under ADR 0009 |
| Section 5 and completed R0–R2 delivery descriptions | Accepted ADRs 0001–0014, [foundation evidence](gallery-foundation.md), and the immutable Git source above |
| R2b preview and conditional interaction acceptance | [Foundation regression requirements](gallery-foundation.md) |
| R2c.1–10 and R2c.12–13 | [Continuity acceptance contract](library-continuity.md), subordinate to current ADRs |
| R2c.11 per-slice remediation chronologies | The stage records above; exact earlier wording and measurements remain in the immutable Git source |
| R3–R10 detailed scope and product value checkpoints | [Organization workflow requirements](../product/organization-workflows.md) |
| Section 7: large-library ladder | The evidence ladder below, with unchanged authorization boundaries |
| Fixed queue, current blockers, required physical splits | [Current roadmap](../roadmap.md#current-execution-queue); detailed historical attribution in the interleaving ledger |
| Bounded investigation, eight scenarios, budgets, cautions, exit rules | [R2c closeout execution](../plans/r2c-closeout.md), transferred as one complete procedure |
| Existing checkpoints, dated closeout narratives and section 10.1 snapshots | Named audit/stage records above plus the immutable Git source; never current-status authority |
| Section 10.2 foundation acceptance checkpoints | [Gallery foundation](gallery-foundation.md#historical-acceptance-checkpoints) |

The source revision is a provenance pointer, not a moving verification claim. No new full roadmap
archive is maintained alongside the canonical file. Preserve initial failures and measured limits
in their owning record; do not turn a historical implementation snapshot into a present-tense claim.

## Large-library evidence ladder

Large testing starts during R1 rather than waiting for R10:

1. deterministic fixtures for corrupt, locked, unavailable, Chinese, long-path, and wrong-extension
   media;
2. synthetic thousands and tens-of-thousands of paths and catalog rows;
3. virtual-gallery stress data large enough to exercise timeline jumps and lazy disposal;
4. controlled read-only scan of `local-primary`;
5. controlled read-only scan of `cloud-primary` after availability checks;
6. controlled read-only combined scan;
7. warm incremental scan after known additions, removals, and modifications;
8. live create, modify, rename, replacement, removal, and event-storm reconciliation during R2c;
9. closed-application journal catch-up, no-change zero-enumeration startup, live-change preemption
   during P2 recovery, and forced journal-gap recovery during R2c;
10. exact-fingerprint reuse, regrouping, cancellation, and target-library duplicate coverage during
    R3;
11. resumable review, override durability, and confidence-band sampling during R4 and R5;
12. perceptual-candidate quality and bounded index evidence during R6;
13. immutable dry-run determinism and unchanged source and target trees during R8;
14. separately authorized operation fixtures and recovery evidence during R9.

Every large run records file counts, duration, throughput, structured issue counts, cancellation
behavior, recovery behavior, peak resource observations where available, cache growth, and whether
source bytes changed.
