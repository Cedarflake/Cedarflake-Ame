# Test design and maintenance

Status: binding test-design requirements under [AGENTS.md](../../AGENTS.md#12-testing-and-verification)

Read this document before adding, changing or removing tests. It owns test value, assertions,
fixtures and consolidation. The [verification workflow](../acceptance/quality/verification-workflow.md)
owns required check categories, execution and delivery; [execution safeguards](../acceptance/quality/execution-safety.md)
own locks, source safety and temporary storage. This document adds no mandatory test count or suite.

## Decide what needs protection

Before adding a test, identify the required behavior, a plausible failure and the gap in existing
coverage. The practical question is: **what relevant error would this test catch that the retained
tests would miss?** Use the answer to choose the owner and scope; a separate document per test is
unnecessary. Reuse an adequate existing case or extend its meaningful input table before adding
another fixture, mock hierarchy or execution path.

Do not add tests for reversible, low-impact edits when compilation, formatting, document checks or
existing behavior coverage establish the result. Pure moves and import changes usually reuse their
current assertions and dependency checks. File length, test count and coverage percentage alone do
not establish a gap. Rare failures still deserve coverage when they can lose source media, durable
decisions or publication authority; speculative or unreachable combinations do not justify a matrix.

| Changed responsibility | Useful evidence |
| --- | --- |
| Domain rule | Observable decisions at meaningful precedence, tie, invalid-input or stale-evidence boundaries |
| Persistence or source adapter | Actual SQLite/file fixtures for transactions, rollback, identity, permissions and placeholder admission |
| Async lifecycle | Controlled cancellation, supersession or failure at the affected ownership boundary, with terminal state and cleanup assertions |
| Composition or user workflow | The production assembly and affected interaction, including its real result and resource retirement |
| Quality gate | Admitted and rejecting examples through the real entrypoint; verify exit status and protected effects, with parser unit cases where needed |
| Documentation or pure relocation | Relevant links, formatting, imports and retained behavior evidence |

## Assert the requirement

Derive expected outcomes from the product contract, accepted invariant, independently known fixture
or reproduced defect. Do not compute the expected value with the implementation under test, copy its
branch structure into the test, or assert only that a mock returns its configured value.

Assert the behavior that would make a failure matter: retained data, rejected stale authority,
published result, released resource or bounded work. Type existence, non-empty output, absence of an
exception and broad snapshots are insufficient when the requirement concerns a more specific result.
Name cases so a failure identifies the scenario and the violated expectation. Explain non-obvious
fixture values through their domain meaning rather than unexplained constants.

Avoid assertions tied only to private helper names, source strings, exact internal call sequences
or incidental object layout. Such details are testable when they are an actual contract, such as
joined shutdown order, a query bound or zero source reads; use the owning requirement to select them.
Test third-party behavior only where Ame relies on an adapter contract, pinned compatibility or
documented failure mode. Do not independently retest ordinary standard-library operations.

## Keep fixtures faithful and bounded

Use fakes for controllable collaborators, clocks and failures. A fake cannot establish SQLite
atomicity, Windows handle sharing, real file identity, bridge mapping or production binding coverage.
Use the existing real fixture or client owner for that claim. Reuse shared fixtures with stable
meaning; avoid a second production implementation inside test helpers.

Prefer explicit signals or barriers to sleeps for ordering. Bound data, waits and cleanup, and retain
the scenario that motivated the boundary. Do not add production flags, callbacks or general-purpose
abstractions solely to make a trivial test possible. Any new seam must have a clear responsibility
and preserve the existing lifecycle under
[ADR 0025](../architecture/0025-invariant-owned-workflow-modules.md).

When assertion strength is uncertain, use the reproduced failing version or a narrowly controlled
fault to check that the relevant case fails for the intended reason. Restore the valid implementation
and run the affected check. Fault injection uses disposable fixtures, never real source media. This
is a targeted diagnostic option, not a requirement to run repository-wide mutation testing or to
create another permanent harness for every change.

## Maintaining and pruning tests

Remove tests for retired requirements, duplicated cases with no distinct failure boundary, mock
self-confirmation and incidental implementation structure. Merge overlapping inputs into a readable
table when they exercise the same invariant. Keep separate cases when different lifetimes,
integration boundaries or failure causes require distinct proof; fewer test functions alone is not
a reason to combine unrelated workflows.

Before removal, identify what remains to catch the relevant failure, or the accepted requirement
change that makes the case obsolete. Preserve meaningful source-safety, durable-data, cancellation,
concurrency, transaction and recovery coverage. Remove unused fixtures and dependencies with their
last consumer. Record the coverage rationale with the owning change, without creating a parallel
test inventory or a report for every deleted assertion.

A failing test needs diagnosis before deletion or weaker expectations. Change the expected behavior
only when its owning requirement or fixture is wrong or obsolete. Repair unreliable ordering and
environment assumptions at their cause; broad ignores, longer waits and removed workloads cannot
substitute for that repair. Existing warning and failure-reporting rules remain in force.

## Select checks and stop

Run checks that answer an unresolved question about the current change, through their existing
execution owner. Preserve valid results for unchanged inputs. Broaden or repeat verification when
changed behavior, a failure, missing coverage or an explicit request justifies it. A new commit,
context resumption or an unrelated document edit alone does not require another full run.

Once the applicable checks establish the affected result, deliver and continue the authorized work.
Do not generate more tests or hypothetical edge cases to prolong an otherwise complete change.
Client and real-library acceptance remain required when the workflow or user request calls for them;
mocked tests do not replace those interactions.
