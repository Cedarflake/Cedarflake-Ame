# Delivery, implementation and review

Status: binding scoped requirements under [quality-gates](../quality-gates.md).

Read this file when its workflow applies. Use the parent index to select shared workflow and
execution safeguards; the roadmap owns current delivery order and acceptance status.

## Delivery and verification

These requirements continue the [repository contract](../../../AGENTS.md#12-testing-and-verification).
Read the applicable workflow and command definitions before selecting or changing verification.
Gate execution and client checks retain the ownership below.
Test additions, replacements and removals follow the
[test-design standard](../../development/testing.md); select evidence before expanding a suite.

### Verification ownership

Every completed change must be supported by verification proportional to its risk. Automated tests,
static checks, and routine builds run through the existing GitHub Actions workflows. Add or update
the boundary regressions needed for the change and commit them with the implementation. Applicable
format, lint, type, unit, integration, build, and acceptance requirements remain in force; their
execution ownership follows this document and its [command reference](commands.md#repository-command-reference).

Before pushing, complete necessary local diagnosis, formatting and format checks for owned files,
final diff inspection, `git diff --check`, and submission-scope review. Verify affected client
workflows locally when needed to establish the actual user result, building the application for
that purpose as necessary. Keep any diagnostic local test focused on the specific unresolved
question. Do not repeat the full checks assigned to CI on the workstation or require a local Daily
run before committing and pushing.

Hosted execution follows the change-scoped [gate selection](gate-selection.md): ordinary
presentation changes do not require native performance workloads. Full-suite and release owners
retain data-safety, migration, recovery and cancellation checks. Do not manually dispatch extended
workloads after each intermediate commit; request them for affected performance evidence or use the
nightly/release owner.

After pushing, check the Actions results for that exact commit under the
[required engineering workflow](#required-engineering-workflow). A result for an earlier commit does
not cover uncommitted or changed inputs. Verify the affected real user path and
source-media safety where applicable, and report outstanding limitations within their actual scope.

Required test categories include, where applicable:

- domain invariant and application use-case tests;
- adapter contract tests using fixed fixtures;
- database migration and rollback-safety tests;
- typed bridge serialization and compatibility tests;
- cancellation, retry, recovery, and partial-failure tests;
- UI state and accessibility tests;
- corrupt, locked, unavailable, Chinese-path, long-path, and wrong-extension media fixtures.

Large-library benchmarks are separate acceptance evidence, not substitutes for correctness tests.
Run necessary local heavyweight builds and benchmarks serially. If an applicable check is blocked,
identify the specific affected result, use a safe relevant alternative where it can answer the
question, and continue authorized work that does not depend on that result. Preserve the remaining
verification gap without repeatedly reopening unaffected work.

### Required engineering workflow

For every material code/configuration change:

1. Recover scope, history, live source, owning ADRs, and `git status`; identify explicit owned files.
   Never mutate, format, stage, or revert unrelated work to pass a gate.
2. State the smallest complete outcome, affected layers, safety constraints, and proving test;
   implement the narrow change with its boundary regressions.
3. Format owned files. Use `quality_format.ps1` only with a clean or wholly owned target tree;
   otherwise format explicit files and run `quality_format.ps1 -Check`.
4. Complete the necessary local diagnosis and affected client verification selected under
   [verification ownership](#verification-ownership).
   Inspect the final diff, generated files, `git diff --check`, and status. Confirm that only owned
   changes belong in the delivery, then stage, commit, and push under
   [Git discipline](../../../AGENTS.md#14-git-discipline).
5. Inspect the hosted workflows for the pushed commit, including its applicable focused regressions,
   lint, Daily, integration, and build gates. Check the run's source commit and actual job results;
   do not attribute another commit's results to this change.
6. If CI fails, read the failing logs and determine whether the cause belongs to the implementation,
   fixtures, test expectations, or execution environment. Correct that cause and push the relevant
   fix while retaining required workloads and behavioral assertions. Reduced requirements, broader
   exclusions, or repeated unchanged reruns must not hide an unresolved failure.

While CI is pending, continue authorized remaining work within the same goal that does not depend
on the pending result, following the independent-delivery boundaries in
[scope and delivery discipline](../../../AGENTS.md#4-scope-and-delivery-discipline). Delivery reports
distinguish what was committed and pushed, the actual client-verification result, and the CI status.
Report pending CI as still running or queued. Include relevant failed, ignored, or unavailable checks,
material risks, and preserved unrelated changes without presenting an unfinished check as passed.

Warnings fail. Never weaken rules, add broad exclusions/ignore comments, or hand-edit generated
output to pass. Fix the cause; a genuinely inapplicable rule permits only a narrow, explained
suppression at its owning configuration. Quality-tool changes must demonstrate passing and
representative rejecting cases where practical. Documentation-only edits use proportionate document
checks; they neither require a new heavy product run nor establish a functional pass.

### Human-readable implementation

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

### Reviewability and behavior-preserving changes

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
