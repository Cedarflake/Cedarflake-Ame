# Project organization after R5

Status: execution specification; the roadmap owns slice disposition.

The [roadmap](../roadmap.md#project-organization-after-r5) selects this maintenance scope after R5.
[ADR 0037](../architecture/0037-feature-module-dependency-boundaries.md) owns the target dependency
rules; the [repository layout](../development/repository-layout.md) owns physical locations, and
[ADR 0025](../architecture/0025-invariant-owned-workflow-modules.md) retains lifecycle boundaries.

## Outcome and scope

Make contracts, use cases, native/platform implementations, production bindings, pages and widgets
independently discoverable. Preserve behavior while making dependency direction enforceable for
future changes. This plan includes its necessary rule checks, tests, imports and documentation.

Keep Flutter/Riverpod, Rust and the generated bridge, source safety, catalog/schema/archive formats,
R4 decisions and the final delivered R5 behavior. Do not redesign the UI, change classification
policy, add packages, split Rust crates or restart completed maintenance work. Retain existing
scan, synchronization, query, viewer and transaction owners. The deferred rendering follow-up stays
separate; this plan does not change its performance requirements or schedule another acceptance cycle.

## Entry and baseline

Begin product edits only after the roadmap closes R5 and its separately authorized merge is complete.
Use that merged source and its valid evidence, not the planning-time R5 branch, as the baseline.
Recover relevant original decisions and recheck live source, status and affected ADRs. Inventory any
changes since planning, including R5 interfaces and runtime factories; the examples below are
discovery points rather than a frozen file roster.

Create `codex/project-organization` from the baseline before implementation. Preserve unrelated
work and keep each independently reversible outcome together. Commit and push each owned slice
before the next independent outcome under the repository Git and quality rules. Merging remains
separately authorized. Planning documents do not start code migration or establish conformance.

## Ordered slices

### O1 Boundary inventory and enforcement foundation

Map each affected interface, implementation, mapping, binding, lifecycle owner, caller and test to
its target location. Include cross-feature imports, generated DTO exposure, barrel exports and
the exact Rust composition functions. Record production, inline-test and dedicated-test sizes for
large affected owners without applying a universal line cap.

The [dependency baseline inventory](project-organization-inventory.md) maps contracts, native mappings,
bindings, existing lifecycle authority, consumers and regression owners to the target locations.

Implement a dependency check in the existing quality workflow with a fixed inventory of legacy
exceptions. New violations fail from this slice onward; each later slice removes its own exceptions.
The final gate must not accept a fresh baseline generated from arbitrary current violations.
Rule configuration and resolved dependencies must follow ADR 0037, including conditional exports
and exact exceptions. Test both admitted and rejecting examples, cycle detection and stale exceptions.

Exit: source/target mapping, named composition owners and an effective check exist; current behavior
is unchanged and no blanket folder exemption hides unclassified handwritten code.

### O2 Platform and storage pilot

Separate `DirectoryPicker`, storage settings, archive pickers and platform actions into application
ports, adapter implementations and typed Provider declarations. Move their concrete bindings to
feature-specific bootstrap modules. Preserve preference persistence and the existing native picker
and Explorer behaviors. Introduce the composition pattern using this bounded slice before broad use.

Exit: production and test assembly use the same contracts; normal startup, required-binding
failure, partial-acquisition cleanup, settings persistence and the affected native actions retain
their results. Tests do not bypass production binding coverage through an alternate all-fake setup.

### O3 Catalog reads and previews

Separate catalog, folder/time/query reads, layout manifests, previews and source-reader adapters.
Move native DTO and error mappings with their implementations. Keep Provider declarations and
application projections with their state owners; remove direct bridge dependencies from these use
cases and presentation. Preserve held-source capabilities and opaque-handle close ownership.

The live mapper dependency closure also includes native scan, duplicate-review, organization,
organization-batch and review implementations. Their existing contracts and bodies move with the
shared asset/query mappings in O3, with required bindings in their named capability modules. O4 and
O5 retain the remaining runtime, service and lifecycle work in their original order; this dependency
closure does not broaden those workflows or duplicate their scheduling/publication owners.

Exit: bounded paging, filtering, time navigation, preview publication and full-resolution viewing
retain their contracts. Adapter fixtures and actual affected browsing paths cover failure, stale
results and source retirement as well as successful reads.

### O4 Scanning and synchronization

Separate scan/synchronization interfaces, adapters and production bindings while retaining the
existing execution, admission, recovery and terminal-publication owners. Keep initial import,
existing-root synchronization and explicit relocation authority distinct. Move resource assembly
without creating a second scheduler, stream subscription or shutdown registry.

Exit: interruption, pause/resume where supported, retry, process close and restart preserve their
existing semantics. Superseded work cannot publish or release a newer resource. Reuse unchanged
source-safe continuity evidence; new lifecycle inputs receive their focused regression coverage.

### O5 Duplicate, organization and classification services

Deliver three independently reversible sub-slices in order:

1. Exact analysis, duplicate queries and duplicate-review adapters and their bindings.
2. Albums, Favorites, review queues, undo/history, archive and cleanup adapters and their bindings.
3. Classification/configuration/read services, model installation and automatic-lifecycle bindings.

Use the final R5 roster. Preserve whole-group targeting, user decisions, archive compatibility,
automatic analysis, installation authorization and restart recovery. Already separated classification
adapters need only affected contract/binding changes; do not reopen their accepted runtime design.

Exit per sub-slice: the concrete implementations and bridge types stay behind ports, production
assembly is covered, and the corresponding retained decisions, failure paths and client workflow pass.

### O6 Presentation organization

Move the unified gallery, duplicate-review and settings page entries into their feature's pages
directory. Group established widgets by capability where helpful. Assign view-only coordinators
by their actual state authority; move application work out of a presentation file only when it
currently owns that work. Keep framework animation, focus, layout and navigation with presentation.

An independent lifecycle extraction requires its own typed seam and behavior comparison under
ADR 0025; a large page alone is not authority for a broad controller rewrite. Update existing UI
verification-map links and tests together with source moves.

Exit: no widget/coordinator depends on a page or concrete external adapter; navigation, retained
scroll/selection, viewer entry/return, focus and shared feedback retain their actual client behavior.

### O7 Repository conformance and handover

Verify Rust domain, ports, application, adapters and API boundaries, including broker/platform
entrypoints and the exact production-composition roster. Resolve aliases and re-exports before
checking dependency direction and cross-layer cycles; the frozen inventory is not the complete
set of paths the final gate must inspect.
Repair a demonstrated ownership violation in its narrow owner, with its regression, rather than
bulk moving native code. Keep generated bridge and canonical tool entrypoint paths stable.

Resolve the native inventory in independently reversible groups:

1. Duplicate source-state probing and duplicate/organization resource composition.
2. Catalog-session, storage and bounded-read boundaries.
3. Classification execution, configuration, authorized installation and shared decision rules;
   preserve transaction-local validation when moving pure rules into domain.
4. Scan admission/publication and preview/source capabilities.
5. Synchronization, journal and recovery integration, including broker protocol translation behind
   Ame contracts, followed by the final exception audit.

Each group distinguishes resource factories from use-case policy, establishes only the missing port
or owner, and retires its exact migration IDs. Retained composition records require a scoped
architecture decision, lifetime, replacement path and regression owner; their neighboring functions
receive no admission. Preserve the inventory's original source and mappings for provenance.

Remove migration-only exports, duplicate bindings and legacy exceptions for completed scope. Move
tests with their owners and update live documentation/tool links; historical evidence retains its
revision and is not rewritten as a current path map. Check that new feature examples can follow the
target without importing implementation details or reviving a retired entrypoint.

Complete the [test-value review](../development/testing.md#maintaining-and-pruning-tests) before
handover. Record retained behavior coverage and actual reductions in the acceptance evidence.

Exit: all target boundaries are covered, remaining exceptions are explicitly accepted technical
exceptions rather than unfinished migration, and applicable submitted-commit CI and client checks
have evidence. Record results in an owning acceptance record and update ADR/roadmap disposition.

## Verification and delivery

Each slice records its affected invariant, public surface, resource lifetime, proof and rollback
boundary. Keep functional behavior changes separate from organizational moves. A pure move retains
existing assertions; add tests where assembly, ownership, failure or race coverage is missing.

Use [quality-gate ownership](../acceptance/quality/verification-workflow.md#required-engineering-workflow): local
format/diff preparation and necessary affected-client checks, then normal hosted checks for the
pushed commit. Use serial lock-aware local commands and the existing controlled test library.
Do not turn directory migration into another full-library import, model evaluation or performance
campaign when the relevant inputs are unchanged.

Evidence must cover no duplicate runtime construction, stable Provider scope/disposal, unchanged
port results/error conversion, preserved cancellation/publication authority and the affected real
interactions. Startup failure and incomplete bindings are part of composition verification.

After the complete migration, launch the resulting desktop client and check regressions using the
configured real library with isolated derived storage. Cover startup and retained browsing, queries,
previews, full-resolution viewing and the affected background workflows. Select bounded checks from
the existing client/source-safety procedures; preserve original media and catalogs, never hydrate
placeholders, and reuse installed resources. Record the actual interactions and failures in the
acceptance record before closing O7.

Revert an owned failing slice with an ordinary commit if required. Revert its bindings, imports,
tests and live documentation together; preserve unrelated work, catalogs, user decisions and media.
Retain the useful evidence and continue only from a coherent baseline. No destructive Git operation,
source mutation, schema reset or release publication is part of this plan.

## Execution baseline and evidence

The authorized branch is `codex/project-organization`, based on merged R5 revision `f74146a4`.
GitHub runs CI without waiting or blocking subsequent slices. PR metadata describes the complete
rollout scope and remains stable between explicitly requested updates. Normal hosted checks continue
independently of scoped local preparation.

The [acceptance record](../acceptance/project-organization.md) owns delivered slices, exact checks,
failures and limitations. The [handover](../acceptance/project-organization-continuity.md#organization-handover--2026-11-04)
maps every slice to evidence and retains the pause-timeout observation's disposition. The
[roadmap queue](../roadmap.md#current-execution-queue) owns remaining work. Completed implementation
details belong in those records rather than accumulating here.
