# Project documentation

Each document has one primary responsibility. Start with the roadmap for current work, then read
only the contracts, execution details, and evidence that own the task.

| Question | Document owner |
| --- | --- |
| What stage is active, what blocks it, and what comes next? | [Roadmap](roadmap.md), the sole active stage and priority index |
| What is the product and which concepts or capabilities are in scope? | [Workbench contract](product/workbench.md) and [organization workflows](product/organization-workflows.md) |
| What are R4's album, favorite, organization-progress, and recovery rules? | [R4 requirements](product/r4-organization.md); control placement remains in the [gallery UI contract](product/gallery/organization.md#review-workflow) |
| Where are the delivered R4 slices defined? | [R4 implementation plan](plans/r4-organization.md) and [roadmap disposition](roadmap.md#r4-implementation-sequence) |
| What are R5's metadata, smart-album, analysis, and recovery rules? | [R5 requirements](product/r5-classification.md), with the accepted [smart-album UI](product/gallery/smart-albums.md#smart-album-workflow) |
| Where are the delivered R5 slices defined? | [R5 implementation plan](plans/r5-classification.md) and [roadmap disposition](roadmap.md#r5-implementation-sequence) |
| What is planned for similar images in Duplicates? | [R6 implementation plan](plans/r6-similar-images.md), including planned interaction; the roadmap owns activation. |
| How are semantic search and composed filters implemented? | [R7 implementation plan](plans/r7-semantic-discovery.md); the [roadmap](roadmap.md#r7-implementation-sequence) owns current slice disposition. |
| What does merged R8 cover, and how will file changes be verified? | [File organization and recovery](product/organization-workflows.md#r8---file-organization-and-recovery) and the [filesystem verification procedure](acceptance/r8-file-operations.md) |
| How will project organization be refactored after R5? | [Organization execution plan](plans/project-organization.md), activated by the [roadmap](roadmap.md#project-organization-after-r5) |
| Where are organization changes and their verification recorded? | [Initial, catalog, scan and preview evidence](acceptance/project-organization.md); [incremental and synchronization evidence](acceptance/project-organization-continuity.md) |
| How should the accepted gallery and controls behave, and why? | [Gallery UI contract and rationale](product/gallery-ui.md#design-principles-and-rationale), under the accepted UI ADRs |
| Which shared UI owner, regression and client path apply to a change? | [UI verification map](acceptance/quality/ui-verification.md#ui-workflow-verification) |
| Where does current maintenance resume? | [Current queue](roadmap.md#current-execution-queue) and its linked active plan |
| How is GPU smart-album analysis selected and verified? | [GPU execution decision](architecture/0039-gpu-classification-execution.md), [settings contract](product/gallery/settings.md) and [GPU evidence](acceptance/gpu-analysis-acceleration.md) |
| Where are completed maintenance outcomes and deferred observations? | [Completed maintenance index](acceptance/completed-maintenance.md), linking the owning evidence |
| Where is the completed R3 delivery recorded? | [R3 exact duplicate implementation](plans/r3-exact-duplicates.md) and its linked verification; the roadmap selects subsequent work |
| Where is the frozen R2c execution history? | [R2c closeout execution](plans/r2c-closeout.md), with its final checkpoint and historical coverage |
| Why was a technical boundary chosen? | [Architecture decisions](architecture/README.md) |
| What must be proved, what actually ran, and what remains unverified? | [Acceptance index](acceptance/README.md) and [quality gates](acceptance/quality-gates.md) |
| Where do implementation and tool files belong? | [Repository layout](development/repository-layout.md) |
| How should documents be written, split and maintained? | [Documentation standard](development/documentation.md), required by `AGENTS.md` |
| Which tests add useful coverage, and which should be merged or removed? | [Test design and maintenance](development/testing.md), required by `AGENTS.md` |
| Which future module dependencies, public contracts and production bindings are allowed? | [ADR 0037](architecture/0037-feature-module-dependency-boundaries.md), with staged adoption through the roadmap |
| Which native factories retain concrete adapters, and who owns their resources? | [Native composition contract](architecture/workflow-ownership/native-composition.md) and its exact dependency registrations |
| Which durable engineering and safety rules apply? | [Repository contract](../AGENTS.md), with its required [delivery/verification workflow](acceptance/quality/verification-workflow.md#delivery-and-verification) |
| Where did the reorganized rules move, and how was their loading checked? | [Rule organization evidence](acceptance/engineering-rules-organization.md) |

Keep stable requirements separate from dated observations. Update an obsolete roadmap status instead
of appending another repair story. Put execution variants and budgets in the linked plan; results,
commands, measurements and failure history in acceptance records; and technical decisions in ADRs.
Historical provenance is retained in the [acceptance index](acceptance/README.md#historical-roadmap-provenance).
Moving text between these owners does not advance a stage, authorize a run, or change product behavior.
