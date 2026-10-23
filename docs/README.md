# Project documentation

Each document has one primary responsibility. Start with the roadmap for current work, then read
only the contracts, execution details, and evidence that own the task.

| Question | Document owner |
| --- | --- |
| What stage is active, what blocks it, and what comes next? | [Roadmap](roadmap.md), the sole active stage and priority index |
| What is the product and which concepts or capabilities are in scope? | [Workbench contract](product/workbench.md) and [organization workflows](product/organization-workflows.md) |
| What are R4's album, favorite, organization-progress, and recovery rules? | [R4 requirements](product/r4-organization.md); control placement remains in the [gallery UI contract](product/gallery-ui.md#review-workflow) |
| In what order should R4 be implemented, and what must each slice deliver? | [R4 implementation plan](plans/r4-organization.md), ordered by the [roadmap](roadmap.md#r4-implementation-sequence) |
| How should the accepted gallery and controls behave, and why? | [Gallery UI contract and rationale](product/gallery-ui.md#design-principles-and-rationale), under the accepted UI ADRs |
| Which shared UI owner, regression and client path apply to a change? | [UI verification map](acceptance/quality-gates.md#ui-workflow-verification) |
| Where does the current maintenance work resume? | [Maintainability refactor execution](plans/maintainability-refactor.md), selected by the roadmap |
| Where is the completed R3 delivery recorded? | [R3 exact duplicate implementation](plans/r3-exact-duplicates.md) and its linked verification; the roadmap selects subsequent work |
| Where is the frozen R2c execution history? | [R2c closeout execution](plans/r2c-closeout.md), with its final checkpoint and historical coverage |
| Why was a technical boundary chosen? | [Architecture decisions](architecture/README.md) |
| What must be proved, what actually ran, and what remains unverified? | [Acceptance index](acceptance/README.md) and [quality gates](acceptance/quality-gates.md) |
| Where do implementation and tool files belong? | [Repository layout](development/repository-layout.md) |
| Which durable engineering and safety rules apply? | [Repository contract](../AGENTS.md), with its required [delivery/verification workflow](acceptance/quality-gates.md#delivery-and-verification) |
| Where did the reorganized rules move, and how was their loading checked? | [Rule organization evidence](acceptance/engineering-rules-organization.md) |

Keep stable requirements separate from dated observations. Update an obsolete roadmap status instead
of appending another repair story. Put execution variants and budgets in the linked plan; results,
commands, measurements and failure history in acceptance records; and technical decisions in ADRs.
Historical provenance is retained in the [acceptance index](acceptance/README.md#historical-roadmap-provenance).
Moving text between these owners does not advance a stage, authorize a run, or change product behavior.
