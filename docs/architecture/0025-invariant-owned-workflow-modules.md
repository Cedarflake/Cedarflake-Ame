# ADR 0025: Keep workflows thin through invariant-owned modules

- Status: Accepted
- Date: 2026-09-06
- Last amended: 2026-10-18

## Context

Phase 34 crossed several real ownership boundaries at once: retained-catalog migration repair,
foreground scan finalization, atomic publication, root removal, physical database reclamation,
preview replacement, and multi-root presentation state. The reported defects required coordinated
changes, but repeatedly extending the existing top-level files also exposed structural debt.

The largest files are not automatically wrong. Some contain stable schema history or extensive
tests. The actionable problem is responsibility growth: a controller, adapter, or orchestrator that
already coordinates several subsystems should not acquire another issue-specific flag, retry loop,
SQL block, or presentation guard. Such patches obscure the owning invariant and make a local fix
silently change unrelated workflows.

## Decision

[ADR 0037](0037-feature-module-dependency-boundaries.md) extends this ownership rule with the future
module dependency and production-composition target. It does not replace the lifecycle and
transaction boundaries below.

Top-level workflow files are composition boundaries. They may translate inputs, select a typed use
case, sequence owned collaborators, and publish a typed outcome. They do not own the detailed state
machine or persistence contract of every collaborator.

New behavior is placed in an invariant-owned module when any of these conditions applies:

- the existing owner already coordinates two or more independent state machines;
- the change needs its own cancellation, retry, transaction, or stale-publication rules;
- the same owner has accumulated fixes for unrelated user-visible symptoms in the current phase;
- the behavior can be tested through a narrower contract than the top-level workflow.

Extraction must move real responsibility. A forwarding wrapper with shared mutable flags or SQL
left in the original file does not establish a boundary. Each extracted module exposes typed inputs
and outcomes, owns its invariants, and has focused tests for success, cancellation, stale work, and
the relevant race or rollback boundary. Issue-specific error codes may describe an invariant
failure; they must not become an alternate control plane spanning presentation, application, and
persistence layers.

### Scoped owner contracts

The following files continue this accepted decision. Before changing a listed subsystem, read its
owner contract alongside the general decision above. Read other owner files only when the change
crosses those boundaries. Keep concrete owner details in those files and new general rules here.

### Rust boundaries

| Affected owners | Scoped contract |
| --- | --- |
| Scan execution and source admission | [Scan execution and source admission](workflow-ownership/scan-and-source-admission.md) |
| Synchronization, journal and recovery | [Synchronization, journal and recovery](workflow-ownership/synchronization-and-journal.md) |
| Catalog, queries, maintenance and migrations | [Catalog, queries, maintenance and migrations](workflow-ownership/catalog-and-migrations.md) |
| Media previews and storage capabilities | [Media previews and storage capabilities](workflow-ownership/preview-and-storage.md) |

### Dart boundaries

| Affected owners | Scoped contract |
| --- | --- |
| Dart scan, source and synchronization lifetimes | [Dart scan, source and synchronization lifetimes](workflow-ownership/dart-scan-and-synchronization.md) |
| Dart gallery queries, paging and image lifetimes | [Dart gallery queries, paging and image lifetimes](workflow-ownership/dart-gallery-and-viewer.md) |
| Dart composition, feedback and interaction lifetimes | [Dart composition, feedback and interaction lifetimes](workflow-ownership/dart-presentation-and-composition.md) |

### Native verification boundaries

Read [native verification ownership](workflow-ownership/native-verification.md) for controlled
processes, fixture retirement, accessibility probes and their evidence protocol.

### Review and roadmap rule

File length is evidence for investigation, not an automatic rewrite trigger. Review records the
responsibilities changed and the narrow owner of every new invariant. If a complete extraction
would materially broaden the active fix, the current change establishes the typed seam, records the
physical split in `docs/roadmap.md`, and adds no further behavior to that debt area until the split
is completed.

## Rejected alternatives


### Continue appending focused patches to the current owner

Rejected. A patch can be locally correct while making cancellation, transaction, and stale-state
rules impossible to reason about together.

### Enforce a universal line-count limit

Rejected. Generated bridge files, schema history, and test fixtures have different reasons for
size. A line cap rewards cosmetic splitting and does not prove ownership.

### Rewrite all large files during the runtime repair

Rejected. That would mix behavior changes with broad churn, weaken reviewability, and risk the
accepted gallery and continuity contracts.

## Consequences

- Runtime fixes require a small amount of explicit composition code and more focused modules.
- Persistence and application races can be tested at their owning boundary instead of only through
  end-to-end fixtures.
- Some historical large files remain. Their staged splits are roadmap work, not hidden claims of
  completion.
- Future changes that extend a recorded debt owner without first establishing its boundary violate
  this decision and the repository contract.

## Verification

- focused tests cover every extracted invariant and its reported race;
- the complete serial Daily gate and Windows integrations pass after extraction;
- an independent final audit checks behavior, layering, duplicate state machines, and regressions;
- `git diff --check` and hosted PR checks pass before Phase 34 closes.
