# Cedarflake Ame Agent Contract

Status: binding repository instructions

## 1. Purpose and document boundaries

This contract owns durable engineering, architecture, dependency, data-safety, verification and Git
rules. Change it only for project-wide constraints, never temporary choices, experiments or status.
Linked requirement sections continue this contract within their stated scope; read and follow them
when the task matches. Keep each rule in its owning section rather than copying it across documents.

[docs/roadmap.md](docs/roadmap.md) is the sole delivery plan for stages, scope, order, blockers,
decomposition and exit decisions. Recover relevant original history, then read the complete roadmap
before a new project session, resumed material work, stage planning/status or delegation. Read its
active execution plan before that scope. If unavailable, report the exact gap before changing scope
or order. Plans cannot override user instructions, this contract, accepted ADRs or verified behavior,
nor maintain competing acceptance status.

Stable requirements belong in `docs/product`, scoped execution in `docs/plans`, technical decisions
in `docs/architecture`, and verification/provenance in `docs/acceptance`. The roadmap links these
owners without copying transcripts, architecture or implementation diaries. Replace superseded
decisions in their owner; retain their reasons and applicable exceptions. Current status requires
live source and evidence.

Goals record outcomes, scope, order and continuing constraints. Update execution/delivery records
with status, commits, measurements, failures and blockers; revise goals for user changes while
preserving explicit decisions.

Use only `local-primary` and `cloud-primary` for real roots in tracked or user-facing documents,
snapshots, publishable logs and commits. Discover mappings only from ignored
`.agents/local-context.toml` (shape: `.agents/local-context.example.toml`); never copy its paths,
account labels or machine identity. It grants no mutation or hydration. If missing, retain logical
names and report exact local execution unavailable.

## 2. Project context

Ame is a local-first desktop workbench for very large local/cloud-backed image libraries without a
second full source copy. Ame owns workflow, domain, catalog, orchestration, user decisions and
presentation; specialized engines sit behind replaceable adapters.

Original media is irreplaceable. Cataloging, browsing and analysis cannot modify it. Move, copy,
rename, recycle and delete are separate later workflows requiring explicit authorization,
current-state revalidation, history and applicable recovery. Never silently change or download sources.

## 3. Instruction and decision precedence

Precedence: current explicit user instruction, this contract, accepted `docs/architecture` ADRs,
repository tool/language configuration, active plan/roadmap, then general conventions.

Before code changes, read this whole contract and owning ADRs, inspect live source and Git status,
preserve unrelated edits, state the smallest complete user outcome, and identify safety, migration,
licensing and performance risks. Report material conflicts before changing scope, safety, licensing
or stable architecture; historical status cannot override live evidence.

### 3.1 Context compaction and continuity

After compaction, resumption or a new session, query this task's recent original history before
material work. Recover decisions, corrections, rejected approaches and open questions. Compare them
and screenshots with instructions, source, ADRs, Git, CI and relevant execution/evidence. Report
scope, safety, licensing or architecture conflicts before dependent edits.

Memory, summaries, handoffs and narration are discovery hints, not authority or completion evidence.
Prefer task-specific originals; check old unfinished items against live state and preserve delivered
work and valid checks. Resumption alone needs no broad investigation or repeated acceptance. If
required history is unavailable, report the gap and seek direction before an irreversible or
materially different decision; do not invent it.

## 4. Scope and delivery discipline

- Deliver the named outcome. Resolve routine implementation and presentation details within accepted
  decisions independently; ask about material product tradeoffs, expanded scope or changed authority.
  Do not add adjacent features or redesign accepted surfaces without approval. Assumptions must be
  reversible and must not materially change product behavior.
- Work directly by default. Delegate only independent, bounded work whose benefit justifies its
  context/quota cost; keep simple edits, lookups and coordination local. No automatic audit fan-out.
  Default to one active delegate, reuse suitable agents and sequence implementation/review.
  Additional concurrent agents require explicit user request. Give narrow context/deliverables,
  prohibit nested agents/tasks/execution chains unless explicitly authorized, and stop replaced or
  duplicate executors. Avoid duplicate investigations, speculative idle work, unnecessary full-history
  copies and repeat reviews without new evidence.
- Evaluate ownership before each independently deliverable outcome. Review implementation, text,
  tests and necessary documentation as one maintainable end-to-end change with independent rollback.
  Commit and push under section 14 before starting the next independent issue; related files need
  not become separate commits. Shells, mocks, screenshots, compilation or code existence alone do
  not complete a workflow.
- Every diagnostic, test, log or document must answer a concrete question that affects the current
  implementation or delivery decision. More records or checks do not measure functional progress.
- Include findings affecting this outcome's correctness, source safety or normal operation in its
  repair. Keep unrelated improvements in subsequent scope, not as new delivery prerequisites.
- Reuse completed implementation and valid evidence. Recheck affected parts when inputs change,
  a failure or reproduction condition appears, coverage is missing, or the user requests it. Once
  applicable verification is complete, deliver and continue in order. While CI is pending, follow
  the [required workflow](docs/acceptance/quality-gates.md#required-engineering-workflow).
- Diagnose the owning cause before architectural replacement or compensating layers. Repair it
  there with a focused boundary regression; presentation guards, retries, status text or
  catch-and-continue must not conceal unresolved application, persistence or platform rules.
  Readability findings do not authorize unrelated cleanup or repository-wide refactors.
- Before extending a multi-responsibility controller, orchestrator, adapter, migration validator or
  workflow script, map its invariant and extract the affected responsibility into a typed, testable
  owner under [ADR 0025](docs/architecture/0025-invariant-owned-workflow-modules.md). Preserve a narrow
  facade; do not append issue-specific flags, callbacks, branches or SQL. Record a larger split in
  the roadmap and add no further behavior to that debt area until its boundary exists. Repeated
  unrelated growth is a stop signal.
- Record production, inline-test and dedicated-test line counts for large affected owners. Cohesion
  and test volume do not excuse poor physical reviewability. Length alone does not justify churn,
  arbitrary caps, forwarding fragments or moving the same monolith.
- Unattended work does not broaden authorization for publication, source mutation, large downloads
  or destructive repository operations. Section 14 owns branch, staging, commit and push authority;
  publishing releases or contacting third parties requires an explicit request.

## 5. Architecture principles

### 5.1 Layer ownership

- **Domain** owns stable entities, invariants, value objects and errors.
- **Application** owns use cases, orchestration, transactions and policy.
- **Ports** expose narrow persistence, analysis, filesystem and platform contracts.
- **Adapters** replace database, media/metadata, comparison/classification, OS and bridge engines.
- **Presentation** renders state through Ame-owned application contracts.

Rust domain/application cannot depend on UI frameworks, generated bridges, webview/widget/OS UI
APIs. Platform/FFI/IPC commands are thin application translations. Presentation cannot own catalog
policy, scan directories, perform analysis, use engine structures or access the catalog DB directly.

### 5.2 Stable contracts

Keep these concepts distinct:

- `LibraryRoot`: configured source and its availability/scan policy.
- `Asset`: logical visual item independent of an absolute path.
- `AssetLocation`: physical file instance belonging to a root.
- `ContentFingerprint`: versioned evidence of exact byte identity.
- `AnalysisRun`: immutable engine execution with versioned parameters.
- `AnalysisResult`: engine evidence associated with an asset or candidate group.
- `UserOverride`: durable intent surviving reanalysis.
- `OperationPlan`: immutable proposal that does not authorize filesystem mutation.

An absolute path cannot be the sole long-term asset identity. Do not overwrite older algorithm/model
results in place. Retain engine identity/version, parameters, applicable confidence, evidence and
analysis-run identity. Third-party types, identifiers, paths, cache formats, global state and errors
cannot cross an adapter into Ame's domain, schema, desktop bridge or presentation contracts.

### 5.3 Replaceability without speculative abstraction

Use ports for credible replacement pressure: decoding, metadata, exact/perceptual comparison,
classification, embeddings, persistence and platform integration. Do not wrap ordinary internal
code for pattern compliance. Adapters need fixed-fixture contract tests and replacement independent
of catalog/presentation rewrites.

### 5.4 Background work

Long-running work must expose structured progress/issues, support cancellation where possible, be
retry-safe and idempotent at the application boundary, and bound concurrency, memory, filesystem
reads and caches. Handle corrupt, locked, missing, renamed and unavailable files; persist resumable
work when the user workflow requires it. One bad file must not fail the library scan. Native codecs,
model runtimes and other high-risk parsers should run behind a recoverable process boundary when a
crash could terminate the desktop application.

## 6. Filesystem and media safety

- Original media is the source of truth; catalogs and analysis data are derived. Modify sources
  only for the exact operation explicitly authorized by the current request, with required checks.
- Never automatically hydrate OneDrive or other cloud-only placeholders.
- Revalidate file identity/state before publishing derived results or executing a reviewed plan.
- Never present partial/failed scans as the last trustworthy completed catalog.
- Keep databases, caches, thumbnails, temporary files and sidecars outside source trees by default.
- Manage catalog data, durable user decisions, operation history, previews, analysis data, temporary
  files and models as separate storage classes. Decisions and history are not disposable cache.
- Cache keys include relevant file identity/state and algorithm/model/version/parameter identity.
  Invalidation must be explicit, testable and limited to rebuildable data.
- Destructive filesystem commands require explicit verified paths, never a workspace root, home,
  unresolved environment variable or broad glob.

## 7. Persistence and migrations

- Application owns persistence semantics; UI owns neither SQL nor schema knowledge.
- Schema changes require forward migrations and migration tests. Normal upgrades preserve catalogs,
  user decisions and history. A forced rescan is allowed only for provably derived data with its
  reason and cost documented.
- Multi-record invariants use transactions and publish completed state atomically.
- Queries have bounded result windows; continuous browsing cannot require every asset or thumbnail
  in memory. Search/analysis indexes and previews remain rebuildable independently of durable data.

## 8. Dependency and open-source policy

Prefer mature specialized capabilities unless measurement justifies reimplementation. Evaluate
license/distribution, adoption, maintenance, releases, issues, documentation, upgrade cost, stable
API/narrow protocol, Windows packaging, Chinese/long paths, volumes, damaged/cloud files, performance,
memory/cache bounds, cancellation latency, failure isolation and contract testability.

Stars alone are insufficient. Reject or isolate unclear licensing, abandonment, UI-bound core
behavior, undocumented globals, unbounded mutation or unacceptable risk. Lap/GPL applications may be
external references only; never copy their code, components, assets, schema or copyrightable
implementation. Keep reference repositories outside Git history. ADRs own accepted versions,
licenses, alternatives, consequences and replacement strategies; this contract is not an inventory.

## 9. Frontend and presentation engineering

An ADR selects the framework/design system, never a prototype or screenshot. For UI work, read the
[accepted design principles](docs/product/gallery-ui.md#design-principles-and-rationale), relevant
surface contract and [verification map](docs/acceptance/quality-gates.md#ui-workflow-verification).
Use `microsoft-ui-ux` and `google-ui-ux` where applicable without expanding scope into theme
replacement, redesign or extra features. Follow the existing design system and approved wording.

Follow the [component implementation requirements](docs/product/gallery-ui.md#component-implementation)
for admission, pinned-SDK proof, framework ownership, bounded rendering and accessibility, and the
[content/action rules](docs/product/gallery-ui.md#content-and-action-wording) for affected text.

## 10. Language and toolchain rules

Read, write and create text as UTF-8, never relying on the system default. Apply framework defaults
only where that framework is present:

- TypeScript: strict mode; no `any`, `@ts-ignore` or unjustified non-null assertions.
- React/Vue component files: `PascalCase`; ordinary TypeScript files: consistent `camelCase` or
  `kebab-case`.
- Dart files: `snake_case`; types: `PascalCase`; variables/functions: `camelCase`.
- Prefer generated typed bridges over hand-maintained loosely typed maps.

### 10.1 Local Flutter toolchain

Before local Flutter work, read and follow the [local toolchain rules](docs/acceptance/quality-gates.md#local-flutter-toolchain):
use the installed SDK and canonical lock-aware commands serially; retain scoped process ownership
and SDK-lock safeguards. This adds no local CI stage.

### 10.2 Rust engineering

- Use stable Rust and the workspace edition/minimum supported version once declared.
- Keep domain errors structured and actionable; do not panic on user-controlled files/paths.
- `unsafe` requires an accepted ADR documenting necessity, safety invariants, focused tests and review.
- Use bounded channels and explicit cancellation. Never hold database transactions, global locks or
  UI callbacks across slow filesystem/model work.
- Keep generated bridges outside domain/application crates. Pass rustfmt and Clippy with warnings denied.

## 11. Formatting, naming, and comments

Repository configuration wins: `.gitattributes` owns line endings/binaries, `.editorconfig` encoding
and whitespace, `analysis_options.yaml` Dart, `rustfmt.toml` Rust format, and `rust/Cargo.toml` lints.
`.vscode/settings.json` is editor integration, not the sole gate. Change owning configuration and
quality commands together; do not duplicate rules. Only narrow generated FRB Dart/build paths may
skip style analysis, never compilation, tests, bridge hashes or release packaging.

- Use two spaces outside Rust and standard rustfmt formatting in Rust.
- TypeScript uses double quotes, no semicolons, trailing commas and a 100-column target where supported.
- Dart follows `dart format` and `flutter_lints` or stricter repository rules.
- Components/types use `PascalCase`; variables/functions follow language conventions; environment
  variables use `SCREAMING_SNAKE_CASE`; booleans normally start with `is`, `has`, `can` or `should`.
- Comment only design intent, invariants, non-obvious constraints or implementation reasons, not
  surface behavior. Comments/documentation must not mention AI generation, prompts, conversations
  or agent identity.

### 11.1 Human-readable implementation

For new/materially changed source, tests, SQL, workflows and diagnostics, follow the
[handwritten implementation requirements](docs/acceptance/quality-gates.md#human-readable-implementation).

## 12. Testing and verification

Before selecting, changing or running checks, follow [delivery and verification](docs/acceptance/quality-gates.md#delivery-and-verification).
It owns mandatory test categories, local/hosted execution, source-safe acceptance and reporting.

### 12.1 Repository quality commands

Use the [command reference](docs/acceptance/quality-gates.md#repository-command-reference) and mandatory
[execution/storage safeguards](docs/acceptance/quality-gates.md#execution-scope-and-storage), including
serial locks, hosted security, real-root authority and safe reuse/cleanup of `build/test`.

### 12.2 Required engineering workflow

Every material code/configuration change follows the [required workflow](docs/acceptance/quality-gates.md#required-engineering-workflow),
including exact-commit CI, cause-level repairs, warning policy and independent work while CI runs.
Documentation-only changes use proportionate document checks without claiming product acceptance.

### 12.3 Reviewability and behavior-preserving changes

Policy rewrites, async lifecycles and scan/persistence extraction follow the
[review requirements](docs/acceptance/quality-gates.md#reviewability-and-behavior-preserving-changes).

## Code Review Rules

Flag obscured policy precedence, unclear async authority and unrelated responsibility growth with
concrete maintenance/correctness impact. Separate reproducible bugs from readability risks; do not
infer causality from length or authorship. Exclude genuine generated output from handwritten-style
findings. Preserve guards and bounded costs; leave routine formatting to tooling.

## 13. Architecture documentation

ADRs own constraining UI, bridge, database, engine, process-isolation, cache and packaging choices.
Include status/date, context/drivers, options, decision, consequences/risks, validation and
replacement/rollback strategy. They explain accepted choices, never stage order or completion status.

## 14. Git discipline

- Inspect status, preserve unrelated work and classify scope/rollback risk before editing; line
  count alone is not the classification. Explicit user branch instructions take precedence.
- Large changes start on `codex/<topic>` before product edits and are committed/pushed there:
  cross-layer/subsystem work, schema/migrations, generated bridge/public contracts, broad refactors,
  material release/infrastructure/architecture changes or another isolated rollback need.
  Never merge into `main` without explicit request.
- Small changes stay on `main` without a new branch. After applicable section 12 preparation,
  staging owned files, a Conventional Commit and direct push are authorized unless the user says
  otherwise. Inspect hosted CI. Resolve wrong-branch/dirty-tree conflicts without moving, committing
  or discarding unrelated work.
- When packaging validated edits, inspect staged scope and `git diff --check`, retain valid results
  and address missing coverage through its execution owner. A commit alone does not justify another
  manually initiated heavy Daily/release/performance/acceptance run; inspect normal push-triggered CI.
- Destructive reset/checkout, history rewriting and releases require explicit authorization;
  ordinary branch/commit/push authority grants none of these.
- Stage explicit files. Use English Conventional Commits with summaries of at most 20 words. Group
  implementation, copy, tests and necessary docs by independently reversible user outcome. Never
  commit caches, models, local catalogs, source-media samples, build outputs, secrets or external
  reference repositories.
- PR bodies neutrally describe current-head modifications. Omit announcements, release notes,
  narratives, repair diaries, reviewer instructions, fixed-bug descriptions, debugging/conversation
  history, agent actions, review rounds, verification/test/CI/performance results, audit/severity/
  zero-finding verdicts, approvals, merge status and readiness claims. Evidence belongs in checks,
  acceptance records or reviews.
- Named-stage PRs include concise `Roadmap scope` linking the canonical roadmap and mapping slices
  to modifications, without completion, evidence, audit or readiness claims.

## 15. Definition of engineering completion

A change is complete when its behavior is connected end to end, owning domain/adapter boundaries
remain intact, and changed handwritten code exposes decisions and lifecycle ownership with suitable
regression or behavior-preservation evidence. Applicable gates must pass through their section 12
owners. Handle relevant failure, cancellation, empty and stale states; verify data/source safety;
document applicable licensing/migration consequences; exclude accidental generated/unrelated changes;
and state remaining limitations accurately. UI delivery includes the affected real interaction and
its consistency with adjacent surfaces, using the linked verification map. Compilation, mocked
content or a screenshot alone is insufficient.
