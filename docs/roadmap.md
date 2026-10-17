# Cedarflake Ame Roadmap

Status: canonical active delivery plan

Last planning update: 2026-10-17

This file owns product delivery order, the current focus, blocking obligations, and stage exit
decisions. Only one stage may be active. Detailed product contracts, execution procedures, architecture,
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
[Organization workflow requirements](product/organization-workflows.md) preserve the full
R3–R10 scope; their existence does not make those capabilities available.

## Delivery sequence

| Stage | User outcome | Current disposition |
| --- | --- | --- |
| R0 | A real Windows picker-to-catalog-to-preview vertical slice | Accepted foundation |
| R1 | Progressive, resumable, multi-root catalog with source preservation | Accepted foundation |
| R2a | Accepted unified gallery and connected-control design | Accepted foundation |
| R2b | Continuous gallery, bounded previews, selection, viewer, and storage lifecycle | Accepted on 2026-08-13; preserve regression boundaries |
| R2c | Change-driven continuity with usable browsing during synchronization | Completed and frozen on 2026-10-16; preserve delivered behavior |
| R3 | Exact duplicates folded with inspectable physical locations and no source mutation | Completed on 2026-10-17; preserve delivered behavior |
| R4 | Favorites, virtual albums, durable decisions, and resumable review sessions | Next planned stage; not started |
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

**R2c is frozen and R3 is completed. R4 is the next planned stage; implementation has not started.**
The [R3 execution record](plans/r3-exact-duplicates.md) and
[closeout evidence](acceptance/r3-exact-duplicates.md#r3-closeout) retain the delivered
[R3 product contract](product/organization-workflows.md#r3---exact-duplicate-understanding),
client verification, source preservation, resource bounds and hosted results.

The R2c product baseline is commit `5ea62d05eb7f84a64a12967fdba20fda9008f371`. The
[frozen closeout checkpoint](plans/r2c-closeout.md#current-execution-checkpoint) and
[final recovery verification](acceptance/r2c-update-checkpoint.md#r2c-freeze-on-2026-10-16)
retain its delivered behavior, hosted result and deferred incident. Previous R2c queue instructions,
paused-R3 statements and candidate-readiness requirements are historical. They do not schedule
another R2c closeout cycle or block the selected R3 work. Historical test records retain their
actual coverage and failures; stage closure does not rewrite those observations.

The earlier Debug process termination during recovery is deferred until it recurs. Retain its
existing log and Windows error report, with the cause unresolved. Do not schedule another crash
investigation or replay from that old incident. A new occurrence can receive a scoped maintenance
fix without reopening the R2c milestone. New source-safety failures or defects that directly prevent
R3's selected workflow are handled at their owning layer under the repository contract.

## Current execution queue

R3 is closed after exact identification, physical-location inspection, both gallery modes,
duplicate review, incremental maintenance and durable choices passed their affected verification.
Preserve its source-read boundaries, deterministic representatives, bounded windows, retained viewer
navigation and accepted workbench behavior. The execution and acceptance records retain the completed
R3-01 through R3-05 deliveries; they are no longer an active repair queue.

The next stage is [R4 virtual organization and review](product/organization-workflows.md#r4---virtual-organization-and-review-foundation).
Its scope remains Favorites and virtual albums, durable decisions and resumable review sessions.
Select its execution plan when that work is requested. Do not restart completed R2c/R3 acceptance or
advance physical file operations, perceptual similarity or release signing through this closeout.

The [maintainability refactor](plans/maintainability-refactor.md) is implemented on six sequential
responsibility branches: historical migration proof and compatibility repair, synchronization runtime
ownership, gallery refresh/dimension lifecycles, retained-page rules, superseded test paths and
release/acceptance script ownership. Native GitHub stack #28 contains draft PRs #22–#27. The plan
links current hosted checks and the completed client comparison, scoped audit and retained
observations. Applicable stack checks must pass before delivery closeout; integration into `main`
remains a human decision. This maintenance precedes R4 and does not reopen R2c/R3 acceptance.

## Frozen R2c ownership

The [R2c closeout plan](plans/r2c-closeout.md) is historical and has no active execution queue.
The detailed runtime, native-client and platform observations remain in the
[acceptance index](acceptance/README.md). The pre-freeze queue is available at the
[frozen source revision](https://github.com/Cedarflake/Cedarflake-Ame/blob/5ea62d05eb7f84a64a12967fdba20fda9008f371/docs/roadmap.md).
Use the existing records when a relevant boundary changes; do not replay old pending instructions
merely because a new session reads them.

ADR [0024](architecture/0024-windows-change-driven-library-continuity.md) continues to own the
synchronization design. The former ADR 0023 model and its older acceptance slices remain historical.
The signed distribution and installer lifecycle retain their separate qualification under
[ADR 0027](architecture/0027-windows-installer-lifecycle.md). Signing, service installation and
Sandbox diagnostics are outside the R3 queue; future changes to those workflows retain their own
scope, verification and authorization. Historical gaps remain documented without becoming
additional R3 entry gates.

## Retained ownership and decomposition obligations

ADR [0025](architecture/0025-invariant-owned-workflow-modules.md) owns module boundaries. Review
the live owner before extending it; a forwarding wrapper or lower line count is not an extraction.
The retained debt constrains changes to the affected owners. It does not schedule a broad R2c
refactor before independent R3 work; extract the boundary needed by the selected delivery.

- Preserve typed ownership of scan execution/publication, mutually exclusive first-import versus
  existing-root journal opening, query/viewport generation, and committed-removal display refresh.
  Already extracted owners are regression boundaries, not another implementation checklist.
- The retained physical split order is shared historical migration validators and exact shrink-only
  compatibility repair, then production synchronization runtime ownership and Live/Journal/Recovery
  lane state machines. Verify remaining work against current files before starting a split.
- Move tests with their owners. Revisit viewer, selection, or sidebar decomposition only when an
  independent lifecycle requires it. Stable migration history, cohesive layout, and shared quality
  or release entrypoints are not split merely for size.
- The catalog's row serialization, physical-identity state reads and source reconciliation have
  dedicated owners. Preserve the reconciliation transaction across observation admission,
  generation allocation and retained-alias updates. This does not reorder the retained migration
  and synchronization-runtime split duties.
- A repair establishes the required typed boundary and records production/inline-test/dedicated-test
  size. If a larger split exceeds the active repair, retain it here as debt and add no further
  behavior to that debt owner until the required split is complete. This does not authorize a broad rewrite
  or expand the active R3 delivery.

## R2c exit and non-negotiable constraints

R2c is closed. The following constraints preserve its behavior while later stages evolve. The
[continuity contract](acceptance/library-continuity.md) retains the invariant and regression cases;
[quality gates](acceptance/quality-gates.md) owns execution commands and hosted/client boundaries.

- Ordinary supported changes converge through the change-driven production path; unchanged
  synchronization startup performs zero source-root enumeration/media opens/inventory creation.
  P0 retains its one-second P95 under P1/P2 work; continuous closed-process single changes retain
  the two-second P95 bound. R3 analysis reads are separately scheduled and accounted for.
- Cached content remains usable during continuity work. No stale result, partial scan, failed
  recovery, unavailable root or interrupted task can publish false completion, freshness or absence.
- Original media, original catalogs and durable user intent remain intact. No routine full scan,
  cloud hydration, catalog reset, weakened identity/lease guard or broader broker privilege may
  hide a failure. Schema changes retain forward migration and data-preservation requirements.
- Reuse the frozen R2c verification and run the applicable regression checks when later changes
  affect its boundaries. The historical final-audit and platform matrices retain their recorded
  results; they are not an active stage-wide acceptance cycle.
- Unsigned Debug/Release remain supported. Signing credentials and signing-dependent installation
  qualification do not become R3 prerequisites. Bounded read-only verification of known roots
  follows the standing repository policy. Source mutation, service installation, release publication
  and branch merging retain their applicable authorization requirements.

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
