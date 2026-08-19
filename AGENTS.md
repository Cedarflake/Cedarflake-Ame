# Cedarflake Ame Agent Contract

Status: binding repository instructions

## 1. Purpose and document boundaries

This file defines durable project rules for agents and contributors. It owns:

- project-wide engineering standards;
- architecture and dependency boundaries;
- data, filesystem, and media safety rules;
- testing, verification, documentation, and Git discipline;
- constraints that must survive session changes and context compaction.

This file must not contain a product roadmap. Do not record milestones, feature order, schedules,
temporary priorities, completion percentages, active experiments, or current implementation status
here. Delivery plans belong in a separate roadmap document. Accepted technical decisions belong in
architecture decision records. Current implementation status is established from the working tree
and verification evidence.

The canonical active delivery plan is the repository-owned `docs/roadmap.md`.
This relative path is a discovery pointer, not roadmap content. Before planning a stage, reporting roadmap
status, resuming material product work after compaction, starting a new project session, or
delegating product work, read that file completely after recovering the latest relevant original
conversation. Do not create a competing roadmap copy. If the file is unavailable, report the exact
continuity gap before changing product scope or stage order. The roadmap remains lower authority
than the user's current instruction, this contract, accepted ADRs, and verified live implementation.

Tracked files refer to the two real large-library roots only as `local-primary` and
`cloud-primary`. Their machine-specific paths belong in the Git-ignored
`.agents/local-context.toml`, whose tracked shape is documented by
`.agents/local-context.example.toml`. When a task requires real-root discovery, read the local
mapping if it exists, but never copy its paths, account labels, or machine identity into tracked
files, test snapshots, logs intended for commit, commit messages, or user-facing documentation.
The mapping is discovery data only: its presence does not grant source mutation, cloud hydration,
or authorization for a new real-library acceptance run. If it is absent, retain the two logical
roots in planning and report that exact local execution is unavailable.

Do not turn a temporary implementation choice into a permanent rule in this file. Amend this
contract only when a project-wide constraint has genuinely changed.

## 2. Project context

Cedarflake Ame is a local-first desktop application for understanding and organizing very large
personal image libraries. It is intended to work safely with multiple local and cloud-backed
directories without requiring a second full copy of the source collection.

The project owns its product workflow, domain model, catalog, task orchestration, user decisions,
and presentation. Mature external libraries may provide specialized capabilities through adapters,
but Ame must remain maintainable when an engine or UI technology is replaced.

Original media is irreplaceable user data. Cataloging, browsing, and analysis must not modify it.
Move, copy, rename, recycle-bin, and delete capabilities remain in product scope as later, separate
workflows with explicit authorization, current-state revalidation, operation history, and recovery
safeguards where applicable. Convenience never justifies silently changing or downloading source
files.

## 3. Instruction and decision precedence

Apply instructions in this order:

1. the user's current explicit instruction;
2. this repository contract;
3. accepted architecture decision records under `docs/architecture/`;
4. repository-owned tool, formatter, linter, and language configuration;
5. the current task plan or roadmap;
6. general conventions.

Before changing code:

1. read this file completely;
2. inspect the working tree and preserve unrelated or user-owned changes;
3. read the architecture records that own the affected area;
4. inspect the live implementation instead of trusting an old status description;
5. state the smallest complete user-visible outcome being changed;
6. identify safety, migration, licensing, and performance risks.

If the user instruction, this contract, an architecture record, and the implementation disagree,
do not resolve a material conflict silently. Report the conflict before changing product scope,
data safety, licensing, or a stable architecture boundary.

### 3.1 Context compaction and continuity

After context compaction, a resumed task, or a new session, do not continue solely from memory, a
compressed summary, an old handoff, or an agent's previous narration. These sources are discovery
hints, not authoritative evidence of the user's latest intent or the implementation state.

Before resuming material work:

1. query the current task's most recent available conversation history using the provided task or
   thread-history tools;
2. identify the user's latest explicit decisions, corrections, rejected approaches, and unresolved
   questions from the original messages rather than relying on a paraphrased recollection;
3. inspect the live working tree, relevant architecture records, and verification results;
4. compare the recovered conversation with the current files and report any contradiction that
   would change scope, safety, licensing, or architecture;
5. resume from verified evidence without recreating completed work or treating an unchecked status
   claim as completion.

Do not edit product code whose behavior depends on pre-compaction decisions until this history
check is complete. A compressed summary may help locate evidence, but it is never decision
authority. Reconcile the original messages with the current explicit instruction, referenced
screenshots, accepted architecture records, and live working tree before acting.

Prefer recent task-specific history over general memory. If the necessary history is unavailable,
state the exact gap and request direction before making an irreversible or materially different
decision. Never fill missing context with a convenient assumption merely to keep work moving.

## 4. Scope and delivery discipline

- Stay aligned with the user's named problem. Do not add adjacent product ideas without approval.
- A delegated subagent must complete its assigned work itself and must not create another subagent,
  child task, peer task, or delegated execution chain unless the user explicitly authorizes nested
  delegation for the current task. The parent agent must state this restriction in every delegation
  prompt, keep the active delegation count bounded, and stop replaced or duplicate executors.
- When a new requirement appears, evaluate the architecture and ownership boundaries first, then implement the smallest maintainable slice; split responsibilities early so a single file does not grow into an unmaintainable monolith.
- Prefer a small end-to-end vertical slice over disconnected backend, UI, or placeholder work.
- Do not count navigation shells, mocked data, screenshots, compilation, or code existence as a
  completed user workflow.
- Diagnose root causes before replacing architecture or adding compensating layers.
- Keep changes narrow. Avoid incidental cleanup and unrelated refactors.
- Make assumptions only when they are reversible and do not materially change product behavior.
- Unattended work does not broaden authorization or permit external publication, source-media
  mutation, large downloads, or destructive repository operations.
- Git branch creation, staging, commits, and pushes follow the standing workflow in section 14.
  Do not publish releases or contact third parties unless explicitly asked.

## 5. Architecture principles

### 5.1 Layer ownership

Keep the system separated into these conceptual layers:

- **Domain**: stable entities, invariants, value objects, and error semantics.
- **Application**: use cases, task orchestration, transactions, and policy.
- **Ports**: narrow contracts for persistence, media analysis, filesystem access, and platform
  capabilities.
- **Adapters**: replaceable implementations for databases, image libraries, metadata tools,
  duplicate engines, classifiers, operating-system integration, and desktop bridges.
- **Presentation**: UI state and rendering based on Ame-owned application contracts.

The Rust domain and application core must not depend on a desktop UI framework, generated bridge
code, webview API, widget toolkit, or operating-system UI API. Platform commands and FFI or IPC
bindings must remain thin translations around application use cases.

The presentation layer must not own catalog policy, scan directories directly, perform analysis, or
depend on third-party engine structures. It must not access the catalog database as an informal
shortcut around the application layer.

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

Create a port where replacement pressure is credible: media decoding, metadata extraction, exact or
perceptual comparison, classification, embeddings, persistence, and platform integration. Do not
introduce interfaces around ordinary internal code merely to satisfy a pattern.

An adapter must be independently testable with fixed fixtures and contract tests. Replacing one
adapter must not require a catalog rewrite or presentation rewrite.

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

Ame should integrate mature capabilities instead of reimplementing specialized algorithms without
a measured reason. A dependency or engine must be evaluated for:

- license and distribution compatibility;
- real-world adoption and credible maintainership;
- release activity, issue quality, documentation, and upgrade cost;
- stable library API or narrow process protocol;
- Windows support and predictable packaging;
- behavior with Chinese and long paths, multiple volumes, damaged files, and cloud placeholders;
- performance, memory use, cache size, cancellation latency, and failure isolation;
- testability behind an Ame-owned contract.

GitHub stars alone are not admission evidence. Reject or isolate dependencies with unclear licenses,
abandoned maintenance, UI-bound core behavior, undocumented global state, unbounded mutation, or
unacceptable operational risk.

Lap and other GPL applications may be inspected as external product and implementation references.
Do not copy their source code, components, assets, schema, or other copyrightable implementation
into Ame. Reference repositories must remain outside Ame's Git history.

Record accepted technology and dependency choices in architecture decision records, including
version, license, alternatives, consequences, and replacement strategy. Do not encode the current
dependency list in this contract.

## 9. Frontend and presentation engineering

The selected UI framework and design system must be recorded in an architecture decision, not
assumed from a prototype or reference application.

Regardless of framework:

- admit UI building blocks in this order: framework and design-system components already in the
  selected stack, repository-owned shared components, mature external packages, then the smallest
  necessary custom layer;
- for every new or substantially redesigned Flutter UI control, inspect the official Material 3
  component catalog at `https://m3.material.io/components`, then verify the corresponding API and
  implementation in the repository-pinned Flutter SDK before writing code; Material design
  availability does not prove that the installed Flutter version exposes every variant or
  configuration;
- record the official component selected, the installed SDK capability that was verified, and any
  remaining product-specific gap in the owning UI decision or task evidence; do not rely only on a
  screenshot, memory, a prototype, or visual similarity;
- before implementing a custom control, record which existing components were evaluated and the
  concrete behavior they could not provide; do not reimplement scrolling, selection, menus,
  dialogs, focus, input, or accessibility behavior already owned by the framework;
- do not admit a third-party UI package merely because it resembles the target design. Apply the
  dependency policy in section 8 and reject stale, low-adoption, poorly documented, or
  difficult-to-replace packages;
- when a product-specific visualization has no complete existing component, compose it around the
  framework primitive that owns interaction and accessibility rather than replacing that primitive;
- render large libraries with virtualization or lazy slivers;
- keep thumbnail decoding and cache use bounded;
- preserve stable item identity and scroll position across incremental updates;
- keep business and persistence state out of view components;
- represent loading, empty, partial, cancelled, failed, and stale states explicitly;
- meet keyboard, focus, contrast, text scaling, and screen-reader accessibility expectations;
- use design tokens and shared components instead of isolated visual constants;
- avoid sending full-resolution images or unbounded result sets across the desktop bridge;
- organize components as behavior first, structure second, and presentation last.

## 10. File encoding

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

## 10. Rust engineering

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

The canonical formatting and lint configuration is:

- `.gitattributes` for repository line-ending normalization and binary classification;
- `.editorconfig` for encoding, line endings, final newlines, and editor-neutral whitespace;
- `analysis_options.yaml` for Dart analyzer language strictness and lints;
- `rustfmt.toml` for Rust formatting;
- the `[lints]` tables in `rust/Cargo.toml` for Rust and Clippy policy;
- `.vscode/settings.json` only for editor integration, never as the sole quality gate.

Do not duplicate these rules in a second formatter or editor-only configuration. When a rule needs
to change, update its owning configuration and the repository quality commands in the same change.
Generated Flutter Rust Bridge Dart sources and transient build output may be excluded from style
analysis only through the narrow paths recorded in `analysis_options.yaml`; they remain subject to
compilation, tests, bridge-hash verification, and release packaging checks.

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

Run quality commands from PowerShell. The scripts resolve Cargo from `PATH` or the user's Rustup
installation and resolve Flutter and Dart from `PATH` or the pinned local SDK location in section
10.1.

Tool scripts use `snake_case` filenames beginning with an ownership category. Do not name a script
only after its immediate action, such as `format.ps1`, `lint.ps1`, `verify.ps1`, or `generate.ps1`.
Use these prefixes:

- `quality_*` for formatting, linting, daily verification, and shared quality implementation;
- `integration_*` for device-backed integration workflows;
- `acceptance_*` for authorization-bound acceptance checks and their guardrail tests;
- `performance_*` for explicit benchmarks and performance evidence;
- `release_*` for packaging and release-candidate verification;
- `bridge_*` for generated bridge maintenance.

GitHub Actions workflow files under `.github/workflows` follow the same `snake_case` ownership
prefixes. Do not add generic names such as `ci.yml`, `build.yml`, `test.yml`, or `release.yml`.
Workflow display names and job IDs must retain the same ownership boundary so branch-protection
checks remain understandable and stable. Reusable workflows are named for the gate they own, not
only for the fact that they are reusable.

Repository workflows must use the narrowest required `GITHUB_TOKEN` permissions, must not use
`pull_request_target` to execute untrusted pull-request code, and must pin external actions to full
commit SHAs. Normal push and pull-request CI may use dependency caches but must not restore compiled
application or library output as trusted build evidence. Authorization-bound real-library paths,
tokens, catalogs, and scans never enter GitHub-hosted workflows.

Choose the narrowest existing category before introducing another one. When a public script is
added or renamed, update its callers, repository instructions, and documentation in the same
change. Do not retain an undocumented alias that creates two canonical entrypoints.

- `./tool/quality_format.ps1` applies `rustfmt` and `dart format` to repository-owned Rust and Dart trees.
- `./tool/quality_format.ps1 -Check` is the non-mutating formatting gate.
- `./tool/quality_lint.ps1` validates repository PowerShell and JSON configuration, runs the formatting
  gate, runs Clippy for all targets and features with warnings denied, and runs the pinned Dart
  analyzer with warnings and informational lints treated as failures.
- `./tool/quality_lint_workflows.ps1 -ActionlintPath <path>` validates all hosted workflows with a
  caller-provided `actionlint` executable. Hosted CI supplies a fixed version with a verified
  checksum; the daily workstation gate does not silently download tools.
- `./tool/quality_test_flutter.ps1` expands the requested widget-test paths and runs each test file
  in its own `flutter test --concurrency=1` process while holding the repository tool lock.
- `./tool/quality_verify_daily.ps1` is the daily gate. It runs the lint gate, Rust tests, Flutter tests, the
  controlled Windows scan integration, the native Windows accessibility integration, generated
  bridge compatibility, and whitespace validation for the complete tracked diff. With no arguments
  it remains serial for workstation safety. The shared hosted workflow may invoke its validated
  `-Component` partitions on isolated runners so the same evidence is collected in parallel without
  sharing Flutter, Cargo, or build state.
- `./tool/integration_test_windows_accessibility.ps1` runs a semantics-enabled virtual-gallery
  stress sequence in the native Windows runner and fails when engine stderr reports an invalid
  `ui::AXTree` update.
- `./tool/quality_verify_git_range.ps1` checks committed whitespace over an explicit Git revision
  range so a clean hosted checkout does not turn `git diff HEAD --check` into an empty gate.
- `./tool/performance_benchmark_synthetic_library.ps1` is the explicit performance gate. It creates 10,000
  temporary images and records cold, warm, pause, resume, memory, and storage evidence.
- `./tool/acceptance_run_read_only_library.ps1` and `./tool/acceptance_verify_read_only_catalog.ps1` are the
  real-library gate. They require current authorization, explicit roots, and storage outside source
  trees; they never become part of unattended daily verification.
- `./tool/acceptance_run_r2c_reliability.ps1` is the R2c-H closeout gate. It first exercises the
  production Windows observer against a disposable source root, then backs up the retained catalog
  into pre-created empty isolated storage and measures catch-up against both explicitly authorized
  roots without publishing authoritative work or reading cloud-placeholder content.
- `./tool/acceptance_test_r2c_reliability_guardrails.ps1` verifies the R2c-H authorization token,
  cloud acknowledgement, path-separation, and fresh-storage boundary without accessing a real
  library.
- `./tool/release_verify_windows.ps1` is the Windows packaging and release-bridge gate. It also
  proves that a packaged process rejects a same-user duplicate before runtime initialization and
  that a replacement starts after the original process exits. Run it when desktop integration,
  native packaging, generated bridge loading, or release behavior changes.
- `./tool/release_verify_candidate.ps1` is the release-candidate orchestrator. It runs the daily, Windows
  release, and synthetic performance gates in order, and adds retained real-library validation only
  when explicitly requested with all authorization-bound paths.
- `./tool/release_validate_version.ps1` requires a `v`-prefixed semantic version and verifies that
  the tag, Flutter application version, and Rust package version agree before a release gate runs.
- `./tool/release_package_portable_windows.ps1` packages the complete Windows x64 Release directory
  as a versioned portable ZIP after release verification.
- `./tool/release_verify_portable_archive.ps1` verifies the portable ZIP filename, single-root
  layout, safe entry paths, and required Flutter and Rust runtime payload without extracting it.

Hosted workflow ownership is:

- `.github/workflows/quality_ci.yml` for pushes to `main`, pull requests, merge queues, and manual
  daily-gate runs;
- `.github/workflows/quality_gate_windows.yml` for the shared Windows daily or release gate;
- `.github/workflows/release_candidate_windows.yml` for version-tag and manual release candidates,
  followed by portable ZIP publication;
- `.github/workflows/release_verify_published.yml` for post-publication attachment verification.

The complete gate definitions and commands are recorded in `docs/acceptance/quality-gates.md`.

Do not replace these entrypoints with remembered command fragments in normal work. A focused test
may be run directly for fast feedback, but it does not replace the applicable repository gate.

### 12.2 Required engineering workflow

For every material code or configuration change:

1. Re-establish scope from the current instruction, recent history when required, `git status`, the
   live implementation, and owning architecture records.
2. Identify unrelated working-tree changes and choose explicit files that this task owns. Never
   mutate, format, stage, or revert another task's files merely to make a gate pass.
3. State the smallest complete outcome, affected layers, safety constraints, and the focused test
   that will prove the behavior before editing.
4. Implement the narrowest maintainable change and add or update tests in the same boundary.
5. Format agent-owned files. Run `./tool/quality_format.ps1` only when the working tree is clean or every
   file it can rewrite belongs to the current task. In a dirty shared tree, format explicit owned
   files with the underlying language formatter, then use `./tool/quality_format.ps1 -Check` as evidence.
6. Run focused tests for the changed behavior, followed by `./tool/quality_lint.ps1`.
7. Run `./tool/quality_verify_daily.ps1` before declaring the change complete. Add the Windows release gate when
   required by section 12.1 and any task-specific integration or acceptance checks.
8. Inspect `git diff --check`, the final diff, generated files, and `git status`. Report exact checks,
   ignored tests, unavailable gates, remaining risks, and unrelated preserved changes.

Warnings are failures. Do not weaken a repository rule, add broad exclusions, use ignore comments,
or hand-edit generated output to make a gate pass. Fix the cause, use the narrowest justified
suppression when the rule is genuinely inapplicable, and record non-obvious exceptions next to the
owning configuration. A quality-tool change must prove both a passing case and that the configured
gate rejects a representative violation where practical.

## 13. Architecture documentation

Use architecture decision records for decisions that constrain future implementation, including UI
frameworks, desktop bridges, database technology, engine selection, process isolation, cache layout,
and packaging.

Each decision record should contain:

- status and date;
- context and decision drivers;
- considered options;
- accepted decision;
- consequences and risks;
- validation evidence;
- replacement or rollback strategy.

Architecture records explain accepted choices. They must not be used as a feature roadmap or a
completion tracker.

## 14. Git discipline

- Inspect the working tree before editing and preserve unrelated changes.
- Classify the requested change as small or large before editing. Scope and rollback risk determine
  the classification; line count alone does not.
- A large change must start on a dedicated `codex/<topic>` branch created before product edits.
  Large changes include work that crosses multiple architectural layers or independent subsystems,
  changes schemas or migrations, changes generated bridge or public application contracts, performs
  a broad refactor, materially changes release or infrastructure behavior, changes an accepted
  architecture boundary, or otherwise needs an isolated rollback boundary. Commit and push large
  changes to that branch. Do not merge the branch into `main` without an explicit user request.
- A small change stays on `main`; do not create a branch for it. After proportionate validation,
  stage only the owned files, create a Conventional Commit, and push `main` directly. This is
  standing authorization to commit and push a requested small repository change unless the user
  explicitly says not to commit or not to push.
- If the current checkout is on the wrong branch, has unrelated changes, or cannot switch safely,
  resolve that state without moving, committing, or discarding unrelated work before applying the
  branch rule.
- When the current task is only to package already-validated working-tree changes into commits,
  keep commit-time verification lightweight. Inspect the staged boundary, run `git diff --check`,
  and add only a focused test when required evidence is missing. Do not rerun daily, release,
  performance, acceptance, or other heavyweight gates merely because a commit is being created.
  A commit request does not invalidate successful verification already obtained for the same diff.
- Do not use destructive reset or checkout operations unless explicitly requested.
- Do not rewrite history or create releases without explicit authorization. The small- and
  large-change workflows above provide authorization only for their stated branch, commit, and push
  operations.
- Stage explicit files rather than broad paths when committing.
- Use concise English Conventional Commit messages with a summary no longer than 20 words.
- Write pull-request bodies as neutral engineering completion records. Include only delivered
  scope and capabilities, stable contracts, verification evidence, and explicit remaining
  boundaries for the current head. Do not use a pull-request body as a bug-fix diary: omit review
  findings, audit iterations, debugging chronology, descriptions of bugs that were fixed, agent
  actions, and conversation history. Do not address end users or use promotional release-note
  language. Keep every statement current and evidence-backed.
- Split unrelated themes into separate commits so each rollback boundary remains coherent.
- Do not commit generated caches, model files, local catalogs, source-media samples, build outputs,
  secrets, or external reference repositories.

## 15. Definition of engineering completion

A change is complete only when:

- its user-visible behavior is connected end to end;
- its owning domain and adapter boundaries remain intact;
- applicable tests and repository quality gates pass;
- failure, cancellation, empty, and stale states are handled where relevant;
- data and source-media safety have been verified;
- licensing and migration consequences are documented when applicable;
- the working tree contains no accidental generated or unrelated changes;
- remaining limitations are stated accurately.

Passing compilation, displaying mocked content, or producing a screenshot is not sufficient by
itself.
