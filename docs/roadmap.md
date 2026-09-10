# Cedarflake Ame Roadmap

Status: canonical active delivery plan

Last planning update: 2026-09-10

This file owns product delivery order, the current focus, blocking obligations, and stage exit
decisions. Only one stage is active. Detailed product contracts, execution procedures, architecture,
and historical verification have separate owners in the [documentation index](README.md).
Read the linked active execution plan before working on its scope; it cannot independently advance
a stage or change this queue. Historical results never establish current-source acceptance.

## Product direction

Ame is a local-first Windows image-organizing workbench for a large personal library. Its sequence
is trustworthy catalog and browsing, exact duplicate understanding, durable virtual organization
and review, machine-assisted understanding, and finally separately authorized physical operations.
The gallery remains one shared canvas throughout. Cataloging, browsing, analysis, and virtual
organization preserve original media and do not require a second full source copy.

The [workbench contract](product/workbench.md) owns product concepts, exclusions, and reference
policy. The [gallery UI contract](product/gallery-ui.md) retains the accepted presentation rules.
[Organization workflow requirements](product/organization-workflows.md) preserve the full future
R3–R10 scope; their existence does not make those capabilities available.

## Delivery sequence

| Stage | User outcome | Current disposition |
| --- | --- | --- |
| R0 | A real Windows picker-to-catalog-to-preview vertical slice | Accepted foundation |
| R1 | Progressive, resumable, multi-root catalog with source preservation | Accepted foundation |
| R2a | Accepted unified gallery and connected-control design | Accepted foundation |
| R2b | Continuous gallery, bounded previews, selection, viewer, and storage lifecycle | Accepted on 2026-08-13; preserve regression boundaries |
| R2c | Change-driven continuity with usable browsing during synchronization | **Active; not accepted** |
| R3 | Exact duplicates folded with inspectable physical locations and no source mutation | Paused until R2c acceptance |
| R4 | Favorites, virtual albums, durable decisions, and resumable review sessions | Planned after R3 |
| R5 | Metadata, primary classification, calibrated confidence, and human correction | Planned after R4 |
| R6 | Perceptual similarity review, distinct from exact duplicates | Planned after R5 |
| R7 | Structured and semantic discovery through the same gallery | Planned after R6 |
| R8 | Immutable physical-organization dry-run with conflicts and no execution | Planned after R7 |
| R9 | Freshly authorized file operations with revalidation, history, and recovery | Planned after R8 |
| R10 | Large-library maturity and release readiness | Later maturity work; infrastructure may support earlier stages |

R2b's [foundation evidence](acceptance/gallery-foundation.md) retains the frozen interaction baseline
and conditional adaptation gates. Accepted work is not reopened merely to complete an optional
migration checklist. R3–R10 retain their full product and safety requirements in the linked contract;
this overview does not reduce them to navigation shells or compilation milestones.

## Current stage and acceptance boundary

**R2c-O — constrained broker and installer foundation remains the active acceptance slice.**
R2c-N admitted the security/protocol foundation. R2c-P and R2c-Q have implementation checkpoints;
R2c-R has controlled local reliability work. Those facts do not accept the slices or all of R2c.
Signed installed-service, real journal, retained-library, and real Cloud Files evidence remain open.
ADR [0024](architecture/0024-windows-change-driven-library-continuity.md) owns the current design;
the former ADR 0023 continuity model and its R2c-I–M results remain historical.

The current implementation includes schema-v32 bounded raw-spool retirement and the 2026-09-09
browsing-recovery correction. Their evidence is in the
[interleaving ledger](acceptance/r2c-interleaving-remediation.md) and
[browsing-recovery record](acceptance/synchronization-browsing-recovery.md).
The latter preserves the initial Daily failure, subsequent focused/partition results, and unsigned
Windows verification. Its isolated mixed-load pass does not establish full-suite stability.
A fresh complete Daily and current retained-library client acceptance remain unproved for that
correction. A documentation reorganization does not change any of these acceptance states.

## Current execution queue

The current direction prioritizes defects that directly impair functions and user experience.
Item 3 selects functional synchronization and browsing. **R2C-C04 now has focused and generated-client
verification** for fresh bulk changes and recovery of retained terminal work. Its original failure
is preserved in the [C04 record](acceptance/r2c-live-gap-recovery.md). **R2C-C05 blocks browsing**:
direct timeline navigation leaves a blank wall, and reported removal/navigation sequences leave
unresolved thumbnails; rail and viewer-return instability also require causal assessment. The
approved browsing diagnosis has stopped at its unknown-cause boundary; the execution plan contains
a proposed native-evidence pivot within the same total ceiling. Correct counts
do not establish usable browsing or complete synchronization acceptance. C01's mixed-load recovery
failure and C02's automation/evidence failures are recorded for final verification; their current
evidence does not establish a highest-priority user incident. Deferral changes execution order,
not their unresolved acceptance status. Source/durable-data harm or reproduced blocked, misleading
or unusable core behavior takes immediate priority. Use the complete
[R2c closeout execution plan](plans/r2c-closeout.md) for scenario selection, investigation budgets,
repair admission, review, verification, stopping rules, and safety cautions.

1. **Locally verified — raw-inventory retirement and bounded cleanup.** Preserve the schema-v32
   separation between execution-authority retirement and bounded, restart-safe physical cleanup.
   The local lifecycle, rollback, stale-execution, complete recovery, reopen, and independent-review
   checkpoint is recorded in the interleaving ledger. Final-source and client acceptance remain
   owned by items 3–4; do not reimplement this completed local prerequisite without new evidence.
2. **Deferred until functional work is complete — mixed-load convergence and latency.** Diagnose the recorded catalog-open,
   observation, publication, and retirement costs through their owning operations. Preserve the
   original 25 P0 samples, P95 at most one second, all 2048 P1 candidates, 10000 P2 source entries,
   4095-entry logical page, and existing deadlines. Exit requires complete P2 results, closing
   journal coverage, final publication, synchronized state, authority retirement, and FULL reopen,
   together with causal correction and the required controlled/hosted latency evidence. Neither
   a first page, diagnostic instrumentation, a later isolated pass, nor a preceding green revision
   closes a failed gate. Historical unexplained attribution stays explicit in the evidence ledger.
3. **Active — functional workflows and Windows client verification.** Cover import/publication,
   interruption/continuation, query/folder consistency, previews/viewer, multi-root update/removal,
   cleanup/registration overlap, and direct navigation/feedback using the frozen eight-scenario
   matrix. Add negative cases for uncovered transitions and preserve the original ten-phase local
   whole-window UIA path and owned-process exit. Real EXE restart and Release decoding/input
   evidence cannot be inferred from widget remounts, static previews, MSAA, or unsigned bridge smoke.
   Use one independent reviewer; unrelated physical decomposition remains tracked debt.
4. **Queued — final-source gates and readiness decision.** Freeze the candidate, resolve admitted
   findings, and complete applicable local Daily/Windows and hosted PR gates. Record implemented,
   focused-verified, final-source-verified, and client-accepted separately. Later behavior changes
   invalidate affected evidence and return to their owning queue item. Documentation alone does
   not require another heavy product gate. No known core-workflow S0/S1 may be waived for readiness.

The execution plan retains the **functional-first continuation, consumed supplement and unresolved P2 follow-up** without
discarding prior local and hosted failures. Its scope is eight journeys with at most
three variants each, one repair batch of at most three root-cause families, and a 16-hour cumulative
active-engineering ceiling. Tool runtimes remain separately bounded and recorded. It permits one
independent review plus one scoped recheck in the original cycle; the supplement explicitly records
its additional diagnosis, repair and review allowances without resetting prior usage. Budget exhaustion, a fourth blocking family, or a
materially different architecture requires an unresolved checkpoint and replanning, not automatic
expansion. Any known core-workflow S0/S1 still blocks readiness even outside the selected roster.
The C04 exception is consumed. The approved C05 extension adds at most 120 active minutes and
raises the cumulative ceiling to 18 hours; all earlier charges and failed-gate limits remain.

New findings enter the existing ledger with reproduction, severity, owning invariant, and blocked
exit. Only a source/data-safety incident or a proven active-exit blocker interrupts order, with the
reason recorded. No symptom patch, repeated unchanged audit, or unrelated optimization becomes a
new delivery item merely because it was noticed during inspection.

## Retained ownership and decomposition obligations

ADR [0025](architecture/0025-invariant-owned-workflow-modules.md) owns module boundaries. Review
the live owner before extending it; a forwarding wrapper or lower line count is not an extraction.

- Preserve typed ownership of scan execution/publication, mutually exclusive first-import versus
  existing-root journal opening, query/viewport generation, and committed-removal display refresh.
  Already extracted owners are regression boundaries, not another implementation checklist.
- The retained physical split order is shared historical migration validators and exact shrink-only
  compatibility repair, then production synchronization runtime ownership and Live/Journal/Recovery
  lane state machines. Verify remaining work against current files before starting a split.
- Move tests with their owners. Revisit viewer, selection, or sidebar decomposition only when an
  independent lifecycle requires it. Stable migration history, cohesive layout, and shared quality
  or release entrypoints are not split merely for size.
- A repair establishes the required typed boundary and records production/inline-test/dedicated-test
  size. If a larger split exceeds the active repair, retain it here as debt and add no further
  behavior to that debt owner until the required split is complete. This does not authorize a broad rewrite
  or waive the bounded cycle's stopping rules.

## R2c exit and non-negotiable constraints

The [continuity acceptance contract](acceptance/library-continuity.md) retains the complete
invariant, failure, race, source-safety, and acceptance checklist. The
[quality gates](acceptance/quality-gates.md) own executable entrypoints and environment boundaries.

- Ordinary supported changes converge through the change-driven production path; no-change startup
  performs zero source-root enumeration/media opens/inventory creation. P0 retains its one-second
  P95 under P1/P2 work; continuous closed-process single changes retain the two-second P95 bound.
- Cached content remains usable during continuity work. No stale result, partial scan, failed
  recovery, unavailable root, or interrupted task can publish false completion, freshness, or absence.
- Original media and durable user intent remain intact. No routine full scan, cloud hydration,
  catalog reset, weakened identity/lease guard, or broader broker privilege may hide a failure.
- All applicable current-source correctness, performance, native interaction, migration, source-
  safety, and final accumulated independent audit obligations must close. A controlled-cycle pass
  does not waive unresolved findings required by ADR 0024's final audit.
- Signed bundle/publisher, installed-service lifecycle, real broker FSCTL, retained roots, and real
  Cloud Files acceptance retain their existing inputs and current authorization. Prepare each
  concrete bounded run before seeking missing authorization. Hosted Server CI, unsigned artifacts,
  and controlled fixtures do not substitute for those gates.
- R3 remains paused. No source-media operation, release publication, or merge into `main` is
  authorized by this roadmap or a historical acceptance record.

## Maintaining this roadmap

Update only stage disposition, the bounded queue, blockers, required decomposition, and exit
decisions here after checking current source and evidence. Replace an obsolete status instead of
appending another dated repair narrative. Put detailed procedures in the linked execution plan,
stable product behavior in product contracts, accepted technical decisions in ADRs, and commands,
test totals, measurements, failed attempts, and completed repair history in acceptance records.

Historical records and the pre-organization source are discoverable from the
[acceptance index](acceptance/README.md#historical-roadmap-provenance). They remain evidence rather
than a second active roadmap. Do not copy their implementation snapshots back into this file or
infer current acceptance from an old passing head.
