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

Do not turn a temporary implementation choice into a permanent rule in this file. Amend this
contract only when a project-wide constraint has genuinely changed.

## 2. Project context

Cedarflake Ame is a local-first desktop application for understanding and organizing very large
personal image libraries. It is intended to work safely with multiple local and cloud-backed
directories without requiring a second full copy of the source collection.

The project owns its product workflow, domain model, catalog, task orchestration, user decisions,
and presentation. Mature external libraries may provide specialized capabilities through adapters,
but Ame must remain maintainable when an engine or UI technology is replaced.

Original media is irreplaceable user data. Read-only behavior is the default, and convenience never
justifies silently changing, downloading, moving, renaming, or deleting source files.

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
- Do not stage, commit, push, publish, release, or contact third parties unless explicitly asked.

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

- The installed Flutter SDK root on this workstation is
  `%USERPROFILE%\develop\flutter`.
- PowerShell does not currently expose `flutter` or `dart` through `PATH`. Invoke the verified
  executables explicitly instead of searching for, downloading, or installing another SDK:
  - Flutter: `%USERPROFILE%\develop\flutter\bin\flutter.bat`
  - Dart: `%USERPROFILE%\develop\flutter\bin\cache\dart-sdk\bin\dart.exe`
- Run Flutter formatting, analysis, tests, and builds serially on this workstation. If a command
  hangs or leaves a tester process behind, stop and inspect that process before starting another
  Flutter command.

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
- Do not use destructive reset or checkout operations unless explicitly requested.
- Do not rewrite history, stage, commit, push, or create releases without current authorization.
- Stage explicit files rather than broad paths when commits are requested.
- Use concise English Conventional Commit messages with a summary no longer than 20 words.
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
