# Cedarflake Ame Agent Contract

Status: binding repository instructions

## 1. Purpose and document boundaries

This contract owns durable engineering, architecture, dependency, data-safety, verification, and Git
rules, including continuity across sessions. Change it only for a project-wide constraint, never
for a temporary implementation choice, milestone, schedule, experiment, priority, or status diary.

[docs/roadmap.md](docs/roadmap.md) is the sole delivery plan: stage order, scope, blockers,
decomposition, and exit decisions. After recovering the relevant original conversation, read it
completely before a new project session, resumed material work, stage planning/status, or delegation.
Read its active execution plan before executing that scope. If unavailable, report the exact gap
before changing scope/order. Plans cannot override current user instructions, this contract,
accepted ADRs, or verified implementation, nor maintain competing acceptance status.

Stable requirements belong in `docs/product`, scoped execution in `docs/plans`, decisions in
`docs/architecture`, and verification/provenance in `docs/acceptance`. The roadmap links these owners;
it must not copy their architecture, transcripts, or implementation diaries. Current status requires
live source and evidence.

Use only `local-primary` and `cloud-primary` for real roots in tracked or user-facing documentation,
snapshots, publishable logs, and commits. For needed discovery read ignored `.agents/local-context.toml`
(shape: `.agents/local-context.example.toml`); never copy its paths, account labels, or machine identity.
The mapping grants no source mutation, hydration, or new real-library run. If missing, retain logical
names and report exact local execution unavailable.

## 2. Project context

Ame is a local-first desktop workbench for very large image libraries across local/cloud-backed
directories, without a second full source copy. Ame owns workflow, domain, catalog, orchestration,
user decisions, and presentation; specialized engines sit behind replaceable adapters.

Original media is irreplaceable. Cataloging, browsing, and analysis cannot modify it. Move, copy,
rename, recycle, and delete remain separate later workflows requiring explicit authorization,
current-state revalidation, history, and applicable recovery. Never silently change or download sources.

## 3. Instruction and decision precedence

Precedence: current explicit user instruction, this contract, accepted `docs/architecture` ADRs,
repository tool/language configuration, active plan/roadmap, then general conventions.

Before code changes, read this whole contract and owning ADRs, inspect live source/status, preserve
unrelated/user edits, state the smallest complete user outcome, and identify safety, migration,
licensing, and performance risks. Report material conflicts before changing scope, data safety,
licensing, or stable architecture; do not silently choose an old status over live evidence.

### 3.1 Context compaction and continuity

After compaction, resumption, or a new session, query the current task's recent original history
before material work. Recover explicit decisions, corrections, rejected approaches, and unresolved
questions; compare them and referenced screenshots with current instructions, live source, owning
ADRs, and verification. Report conflicts affecting scope, safety, licensing, or architecture.

Memory, summaries, handoffs, and prior narration are discovery hints, not decision authority or
completion evidence. Do not edit behavior dependent on earlier decisions until this check is complete
or recreate completed work. Prefer task-specific originals over general memory. If required history
is unavailable, state the exact gap and seek direction before an irreversible or materially different
decision; do not fill it with assumptions.

## 4. Scope and delivery discipline

- Stay aligned with the user's named problem. Do not add adjacent product ideas without approval.
- Work directly by default. Delegate only an independent, bounded task whose benefit justifies its
  context/quota cost; simple edits, lookups, and coordination stay local. No automatic audit fan-out.
  Default to one active delegate, reuse suitable agents, and sequence implementation/review.
  Additional concurrent agents require explicit user request, regardless of available slots.
- Give only relevant context and a narrow deliverable; avoid duplicate investigations, speculative
  idle work, full-history copies without need, or repeat reviews without new evidence. Every
  delegation must prohibit nested agents/tasks/execution chains unless explicitly authorized for
  this task. The parent enforces that limit and stops replaced or duplicate executors.
- Evaluate ownership before implementing the smallest maintainable end-to-end slice. Shells,
  mocked data, screenshots, compilation, and code existence do not complete a user workflow.
- Diagnose the owning cause before architectural replacement or compensating layers. Keep fixes
  narrow; a readability finding does not authorize unrelated cleanup or a repository-wide refactor.
- Before extending a multi-responsibility controller, orchestrator, adapter, migration validator,
  or workflow script, map its invariant and extract the affected responsibility into a typed,
  independently testable owner. Do not append issue-specific flags, callbacks, branches, or SQL.
  Preserve a narrow facade; record a larger split in the roadmap and add no further behavior to
  that debt area until its boundary exists. Repeated unrelated growth is a stop signal.
- Record production, inline-test, and dedicated-test line counts for large affected owners.
  Cohesion or many tests does not excuse poor physical reviewability. File length alone does not
  justify churn, arbitrary caps, forwarding fragments, or relocating the same monolith.
- Remove the cause at its layer and add a focused boundary regression. Presentation guards, retries,
  status text, or catch-and-continue must not conceal unresolved application/persistence/platform rules.
- Make assumptions only when they are reversible and do not materially change product behavior.
- Unattended work does not broaden authorization or permit external publication, source-media
  mutation, large downloads, or destructive repository operations.
- Git branch creation, staging, commits, and pushes follow the standing workflow in section 14.
  Do not publish releases or contact third parties unless explicitly asked.

## 5. Architecture principles

### 5.1 Layer ownership

Preserve these owners:

- **Domain**: stable entities, invariants, value objects, and errors.
- **Application**: use cases, orchestration, transactions, and policy.
- **Ports**: narrow persistence, analysis, filesystem, and platform contracts.
- **Adapters**: replaceable database, media/metadata, comparison/classification, OS, and bridge engines.
- **Presentation**: state/rendering through Ame-owned application contracts.

Rust domain/application cannot depend on UI frameworks, generated bridges, webview/widget/OS UI
APIs. Platform/FFI/IPC commands are thin application translations. Presentation cannot own catalog
policy, scan directories, perform analysis, use engine structures, or directly access the catalog DB.

### 5.2 Stable contracts

At minimum, keep these concepts distinct:

- `LibraryRoot`: a configured source and its availability or scan policy.
- `Asset`: a logical visual item independent of one absolute path.
- `AssetLocation`: one physical file instance belonging to a root.
- `ContentFingerprint`: versioned evidence of exact byte identity.
- `AnalysisRun`: one immutable engine execution with versioned parameters.
- `AnalysisResult`: engine evidence associated with an asset or candidate group.
- `UserOverride`: durable user intent that survives reanalysis.
- `OperationPlan`: an immutable proposal that does not itself authorize filesystem mutation.

Do not use an absolute path as the sole long-term asset identity. Do not overwrite results from an
older algorithm or model in place. Engine identity, engine version, parameters, confidence where
applicable, evidence, and analysis-run identity must remain traceable.

Third-party types, identifiers, paths, cache formats, global state, and error types must not cross an
adapter into Ame's domain, persistence schema, desktop bridge, or presentation contracts.

### 5.3 Replaceability without speculative abstraction

Create ports for credible replacement pressure: decoding, metadata, exact/perceptual comparison,
classification, embeddings, persistence, and platform integration. Do not wrap ordinary internal
code for pattern compliance. Adapters need fixed-fixture contract tests and independent replacement
without catalog or presentation rewrites.

### 5.4 Background work

Long-running tasks must be:

- observable through structured progress and issue reporting;
- cancellable where the underlying operation permits it;
- safe to retry and idempotent at the application boundary;
- bounded in concurrency, memory, filesystem reads, and cache growth;
- resilient to corrupt, locked, missing, renamed, and unavailable files;
- resumable when persistence is required by the user workflow.

One bad media file must not fail an entire library scan. Native codecs, model runtimes, and other
high-risk parsers should run behind a recoverable process boundary when a crash could terminate the
desktop application.

## 6. Filesystem and media safety

- Treat original media as the source of truth and all catalogs or analysis data as derived.
- Do not modify source media unless the current user request explicitly authorizes the exact
  operation and the implementation has the required safety checks.
- Do not automatically hydrate OneDrive or other cloud-only placeholders.
- Revalidate file identity and state before publishing derived results or executing a reviewed plan.
- Never present a partial or failed scan as the last trustworthy completed catalog.
- Never place databases, caches, thumbnails, temporary files, or sidecars inside source trees by
  default.
- Keep catalog data, user decisions, operation history, previews, analysis data, temporary files,
  and models as separately managed storage classes.
- User decisions and operation history are durable data, not disposable cache.
- Cache keys must include the relevant file identity and state plus algorithm, model, version, and
  parameter identity.
- Cache invalidation must be explicit, testable, and limited to rebuildable data.
- Destructive filesystem commands must use explicit, verified paths. Never target a workspace root,
  home directory, unresolved environment variable, or broad glob.

## 7. Persistence and migrations

- The application layer owns persistence semantics; the UI does not own SQL or schema knowledge.
- Schema changes require forward migrations and migration tests.
- Normal upgrades must preserve catalogs, user decisions, and operation history.
- A forced rescan is acceptable only for provably derived data, with the cost and reason documented.
- Use transactions for multi-record invariants and publish completed state atomically.
- Design queries for bounded result windows. A visually continuous library must not require loading
  every asset or thumbnail into memory.
- Search indexing, analysis indexes, and previews must remain rebuildable independently from durable
  user data.

## 8. Dependency and open-source policy

Ame uses mature specialized capabilities unless measurement justifies reimplementation. Evaluate:

- licensing/distribution, adoption, maintainership, releases, issues, documentation, and upgrade cost;
- stable API/narrow protocol, Windows packaging, Chinese/long paths, volumes, damaged/cloud files;
- performance, memory/cache bounds, cancellation latency, failure isolation, and contract testability.

Stars alone prove nothing. Reject or isolate unclear licensing, abandonment, UI-bound core behavior,
undocumented globals, unbounded mutation, or unacceptable risk. Lap/GPL applications may be inspected
as external references; never copy code, components, assets, schema, or copyrightable implementation.
Keep reference repos outside Git history. ADRs own accepted version, license, alternatives,
consequences, and replacement strategy; this contract is not a dependency inventory.

## 9. Frontend and presentation engineering

An ADR selects framework/design system, never a prototype or reference screenshot.

- Prefer installed framework/design-system components, shared repository components, mature
  packages, then minimal custom code, in that order.
- Before a new/substantially redesigned Flutter control, inspect `https://m3.material.io/components`
  and its API/implementation in the pinned SDK. Record the official component, verified installed
  capability, and product gap in its decision/evidence. Design availability, memory, and visual
  similarity do not establish SDK support.
- Before custom controls, record evaluated components and missing behavior. Do not replace framework
  scrolling, selection, menus, dialogs, focus, input, or accessibility. Compose product-specific
  visualizations around the owning framework primitive.
- Apply section 8 to UI packages; appearance does not justify stale, low-adoption, poorly documented,
  or difficult-to-replace dependencies.
- render large libraries with virtualization or lazy slivers;
- keep thumbnail decoding and cache use bounded;
- preserve stable item identity and scroll position across incremental updates;
- keep business and persistence state out of view components;
- represent loading, empty, partial, cancelled, failed, and stale states explicitly;
- meet keyboard, focus, contrast, text scaling, and screen-reader accessibility expectations;
- use design tokens and shared components instead of isolated visual constants;
- avoid sending full-resolution images or unbounded result sets across the desktop bridge;
- organize components as behavior first, structure second, and presentation last.

## 10. Language and toolchain rules

- Read, write, and create text files using UTF-8 consistently; do not rely on the system default encoding.

Framework-specific defaults apply only when that framework is present:

- TypeScript must use strict mode without `any`, `@ts-ignore`, or unjustified non-null assertions.
- React or Vue component files use `PascalCase`; ordinary TypeScript files use one consistent
  `camelCase` or `kebab-case` convention.
- Dart files use `snake_case`, types use `PascalCase`, and variables and functions use `camelCase`.
- Prefer generated, typed bridge contracts over hand-maintained loosely typed maps.

### 10.1 Local Flutter toolchain

- The installed Flutter SDK root on this workstation is resolved from
  `$env:USERPROFILE\develop\flutter`; do not commit the expanded user-specific path.
- PowerShell does not currently expose `flutter` or `dart` through `PATH`. Invoke the verified
  executables explicitly instead of searching for, downloading, or installing another SDK:
  - Flutter: `& "$env:USERPROFILE\develop\flutter\bin\flutter.bat"`
  - Dart: `& "$env:USERPROFILE\develop\flutter\bin\cache\dart-sdk\bin\dart.exe"`
- Run Flutter formatting, analysis, tests, and builds serially on this workstation. If a command
  hangs or leaves a tester process behind, stop and inspect that process before starting another
  Flutter command.
- Use the repository's lock-aware quality entrypoints for Flutter work. Run focused widget tests
  through `./tool/quality_test_flutter.ps1 -TestPath <path>` instead of launching a parallel
  `flutter test` process. Never terminate Dart or Flutter processes merely because they appeared
  after a test began; cleanup must be limited to descendants or executables proven to belong to the
  current command.
- In a workspace-only agent sandbox, `flutter.bat` must still update its SDK-owned
  `bin/cache/flutter.bat.lock`. If the batch process loops before creating a Dart child, rerun the
  same scoped repository command with the required sandbox approval. Do not delete the SDK lock
  file or infer that an unrelated Dart process owns it without an exclusive-open check.

### 10.2 Rust engineering

- Use stable Rust and follow the workspace edition and minimum supported version once declared.
- Keep domain errors structured and actionable. Do not panic on user-controlled files or paths.
- `unsafe` is forbidden unless an accepted architecture decision documents why it is necessary,
  defines the safety invariants, and adds focused tests and review requirements.
- Use bounded channels and explicit cancellation for concurrent pipelines.
- Do not hold database transactions, global locks, or UI callbacks across slow filesystem or model
  operations.
- Keep generated bridge code outside the domain and application crates.
- Rust code must pass formatting and Clippy with warnings denied.

## 11. Formatting, naming, and comments

Repository configuration takes precedence over these defaults.

Canonical owners are `.gitattributes` (line endings/binaries), `.editorconfig` (encoding/whitespace),
`analysis_options.yaml` (Dart), `rustfmt.toml`, and `rust/Cargo.toml` `[lints]` (Rust/Clippy).
`.vscode/settings.json` is editor integration, never the sole gate. Do not duplicate these rules;
change the owning configuration and quality commands together. Only narrow generated FRB Dart/build
paths in `analysis_options.yaml` may skip style analysis, never compilation, tests, bridge hashes,
or release packaging.

- Use UTF-8.
- Use two-space indentation outside Rust and standard `rustfmt` formatting in Rust.
- TypeScript uses double quotes, no semicolons, trailing commas, and a 100-column target when the
  configured formatter supports it.
- Dart follows `dart format` and `flutter_lints` or the repository's stricter analysis rules.
- Components and types use `PascalCase`; variables and functions use language-idiomatic naming.
- Environment variables use `SCREAMING_SNAKE_CASE`.
- Boolean names should normally begin with `is`, `has`, `can`, or `should`.
- Add comments only for design intent, invariants, non-obvious constraints, or implementation
  reasons. Do not narrate obvious code behavior.
- Comments and documentation must not mention AI generation, prompts, conversations, or agent
  identity.

### 11.1 Human-readable implementation

These rules cover new or materially changed handwritten source, tests, SQL, workflows, and diagnostic
scripts. Only reproducible generator output qualifies for generated-code treatment.

- Optimize for understanding and maintenance, not character, line, or token count. Use normal
  formatting and domain names; short local loop/coordinate names are acceptable when unambiguous.
- A simple two-way conditional is acceptable. Express ordered policies and multiple business
  outcomes as explicit branches or switches; do not compress them into nested ternaries or boolean
  chains. Preserve evaluation order, short-circuiting, side effects, errors, and resource lifetimes.
- Extract named decisions with cohesive inputs/outcomes, not forwarding chains or generic mutable
  state bags. Use named records/types when boundary values have distinct meanings; avoid wrappers
  with no invariant. Follow ADR 0025 before expanding an existing multi-responsibility owner.
- Keep query, publication, source, root, run, and lease identities distinct. Async operations must
  expose result-admission authority, loading ownership, and terminal cleanup. A stale completion or
  `finally` cannot clear a newer operation. Fewer fields are not proof of a simpler state machine.
- Format SQL by its relational decisions; preserve NULL semantics, transaction/identity checks,
  bounded reads, and query-plan costs. Do not introduce per-item queries for cosmetic clarity.
- Explain non-obvious invariants, not surface syntax. Never remove guards, swallow errors, add
  retries, weaken assertions, or change behavior silently under a readability label.

## 12. Testing and verification

Every completed change must be supported by evidence proportional to its risk.

Before declaring a slice complete:

1. run focused tests for the changed behavior;
2. run applicable format, lint, type, unit, integration, and build checks defined by the repository;
3. verify the real user path, not only isolated functions;
4. confirm source media was not mutated;
5. run `git diff --check`;
6. report remaining limitations and any blocked verification honestly.

Required test categories include, where applicable:

- domain invariant and application use-case tests;
- adapter contract tests using fixed fixtures;
- database migration and rollback-safety tests;
- typed bridge serialization and compatibility tests;
- cancellation, retry, recovery, and partial-failure tests;
- UI state and accessibility tests;
- corrupt, locked, unavailable, Chinese-path, long-path, and wrong-extension media fixtures.

Large-library benchmarks are separate acceptance evidence, not substitutes for correctness tests.
Run heavyweight builds and benchmarks serially on this workstation. If a full check is blocked, run
the strongest safe alternative and state the exact unverified gap.

### 12.1 Repository quality commands

Before selecting, changing, or running verification, read the applicable definitions and
[command reference](docs/acceptance/quality-gates.md#repository-command-reference). That document
owns tool parameters, guardrails, environment requirements, and hosted workflow ownership.
Use its canonical PowerShell entrypoints and ownership-prefixed script/workflow names; update
callers and documentation together when an entrypoint changes. Do not create a second canonical alias.

Keep local heavy work serial under the repository lock. Focused tests do not replace applicable
Daily, native, performance, release, or acceptance gates. Real-library runs require current explicit
authorization, roots, and isolated derived storage; they never join unattended Daily verification.
Hosted workflows use minimal token permissions, full-SHA action pins, no untrusted
`pull_request_target` execution, and no restored compiled output as trusted build evidence.
Real-library paths, tokens, catalogs, and scans never enter hosted workflows.

### 12.2 Required engineering workflow

For every material code/configuration change:

1. Recover scope, history, live source, owning ADRs, and `git status`; identify explicit owned files.
   Never mutate, format, stage, or revert unrelated work to pass a gate.
2. State the smallest complete outcome, affected layers, safety constraints, and proving test;
   implement the narrow change with its boundary regressions.
3. Format owned files. Use `quality_format.ps1` only with a clean or wholly owned target tree;
   otherwise format explicit files and run `quality_format.ps1 -Check`.
4. Run focused tests, `quality_lint.ps1`, then `quality_verify_daily.ps1` before completion, plus
   applicable Windows release, integration, and acceptance gates from section 12.1.
5. Inspect the final diff, generated files, `git diff --check`, and status. Report exact checks,
   ignored tests, unavailable gates, risks, and preserved unrelated changes.

Warnings fail. Never weaken rules, add broad exclusions/ignore comments, or hand-edit generated
output to pass. Fix the cause; a genuinely inapplicable rule permits only a narrow, explained
suppression at its owning configuration. Quality-tool changes must demonstrate passing and
representative rejecting cases where practical. Documentation-only edits use proportionate document
checks; they neither require a new heavy product run nor establish a functional pass.

### 12.3 Reviewability and behavior-preserving changes

- State whether a change fixes behavior or only clarifies it. For policy rewrites, enumerate branch
  precedence, ties, nulls, and fallbacks before editing; use boundary examples, not tests mirroring
  the new implementation. Refactoring alone does not close a reported runtime defect.
- Review async ownership across success, failure, cancellation, disposal, supersession, and late
  completion. Retain independent generations and prove the changed boundary's relevant races.
- For scan/persistence extraction, account for checkpoints, old-record preservation, identity and
  metadata reuse, atomic publication, rollback, and source safety. Do not mix broad cleanup with a fix.
- Distinguish maintenance findings from reproduced functional failures. Record affected responsibility,
  causal evidence, behavior risks, focused verification, and unverified paths. Formatting/analyzer
  success alone does not prove readability or behavioral equivalence. New quality gates require
  positive and negative checks; do not add blanket complexity limits or suppress existing warnings.

## Code Review Rules

Flag obscured policy precedence, unclear async authority, and unrelated responsibility growth with
their concrete maintenance or correctness impact. Separate reproducible bugs from readability risks;
do not infer causality from length or authorship. Exclude genuine generated output from handwritten
style findings. Preserve guards and bounded costs; leave routine formatting enforcement to tooling.

## 13. Architecture documentation

ADRs own constraining choices: UI, bridge, database, engines, process isolation, cache, and packaging.
Include status/date, context/drivers, options, decision, consequences/risks, validation, and
replacement/rollback strategy. They explain accepted choices, never stage order or completion status.

## 14. Git discipline

- Inspect status, preserve unrelated work, and classify scope/rollback risk before editing; line
  count alone is not the classification. Follow an explicit user branch instruction first.
- Large changes start on `codex/<topic>` before product edits and are committed/pushed there:
  cross-layer/subsystem work, schema/migrations, generated bridge/public contracts, broad refactors,
  material release/infrastructure/architecture changes, or another isolated rollback need.
  Never merge into `main` without explicit request.
- Small changes stay on `main` without a new branch. Proportionate validation, staging owned files,
  a Conventional Commit, and direct push are authorized unless the user says not to commit/push.
  Resolve wrong-branch/dirty-checkout conflicts without moving, committing, or discarding unrelated work.
- For packaging already-validated edits, inspect staged scope and `git diff --check`; run a focused
  test only for missing evidence. Commit creation alone invalidates no passing evidence and does
  not justify another heavy Daily/release/performance/acceptance run.
- Destructive reset/checkout, history rewriting, and releases require explicit authorization;
  ordinary branch/commit/push authority grants none of these.
- Stage explicit files. Use English Conventional Commits, summaries at most 20 words, and separate
  unrelated themes into coherent rollback commits. Never commit caches, models, local catalogs,
  source-media samples, build outputs, secrets, or external reference repositories.
- PR bodies are neutral descriptions of current-head modifications, not announcements, release
  notes, narratives, repair diaries, or reviewer instructions. Omit fixed-bug descriptions,
  debugging/conversation history, agent actions, review rounds, verification/test/CI/performance
  results, audit/severity/zero-finding verdicts, approvals, merge status, and readiness claims.
  Evidence belongs in checks, acceptance records, or reviews.
- Named-stage PRs include a concise `Roadmap scope` linking the canonical roadmap and mapping
  slices to modifications, without completion, evidence, audit, or readiness claims.

## 15. Definition of engineering completion

A change is complete only when:

- its user-visible behavior is connected end to end;
- its owning domain and adapter boundaries remain intact;
- changed handwritten code exposes its decisions and lifecycle ownership, with appropriate
  behavior-preservation or regression evidence;
- applicable tests and repository quality gates pass;
- failure, cancellation, empty, and stale states are handled where relevant;
- data and source-media safety have been verified;
- licensing and migration consequences are documented when applicable;
- the working tree contains no accidental generated or unrelated changes;
- remaining limitations are stated accurately.

Passing compilation, displaying mocked content, or producing a screenshot is not sufficient by
itself.
