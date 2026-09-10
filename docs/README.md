# Project documentation

Each document has one primary responsibility. Start with the roadmap for current work, then read
only the contracts, execution details, and evidence that own the task.

| Question | Document owner |
| --- | --- |
| What stage is active, what blocks it, and what comes next? | [Roadmap](roadmap.md), the sole active stage and priority index |
| What is the product and which concepts or capabilities are in scope? | [Workbench contract](product/workbench.md) and [organization workflows](product/organization-workflows.md) |
| How should the accepted gallery and controls behave? | [Gallery UI contract](product/gallery-ui.md), under the accepted UI ADRs |
| How is the current bounded investigation executed? | [R2c closeout execution](plans/r2c-closeout.md), subordinate to the roadmap queue |
| Why was a technical boundary chosen? | [Architecture decisions](architecture/README.md) |
| What must be proved, what actually ran, and what remains unverified? | [Acceptance index](acceptance/README.md) and [quality gates](acceptance/quality-gates.md) |
| Where do implementation and tool files belong? | [Repository layout](development/repository-layout.md) |
| Which durable engineering and safety rules apply? | [Repository contract](../AGENTS.md) |

Keep stable requirements separate from dated observations. Update an obsolete roadmap status instead
of appending another repair story. Put execution variants and budgets in the linked plan; results,
commands, measurements and failure history in acceptance records; and technical decisions in ADRs.
Historical provenance is retained in the [acceptance index](acceptance/README.md#historical-roadmap-provenance).
Moving text between these owners does not advance a stage, authorize a run, or change product behavior.
