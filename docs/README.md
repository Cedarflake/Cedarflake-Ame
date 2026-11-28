# Project documentation

Start with the roadmap for current work, then read the contract or procedure that owns the task.

| Question | Document owner |
| --- | --- |
| What remains to deliver? | [Roadmap](roadmap.md), [Beta plan](plans/r10-beta-readiness.md) and [Store distribution plan](plans/store-msix.md) |
| What does Ame do? | [Workbench](product/workbench.md) and [organization workflows](product/organization-workflows.md) |
| How should the interface behave? | [Gallery UI](product/gallery-ui.md) and its scoped surface contracts |
| How do albums, favorites and organization work? | [Organization requirements](product/r4-organization.md) |
| How do smart albums and classification work? | [Classification requirements](product/r5-classification.md) and [smart-album UI](product/gallery/smart-albums.md) |
| How are similar-image groups formed? | [Similarity decision](architecture/0038-bounded-perceptual-similarity.md) |
| How do search and filters work? | [Discovery decision](architecture/0040-durable-discovery-features.md) and [filter decision](architecture/0033-structured-gallery-filters.md) |
| How are file operations and recovery bounded? | [File-operation decision](architecture/0042-windows-file-operations.md) and [verification procedure](acceptance/r8-file-operations.md) |
| How are installation, updates and Store distribution handled? | [Installer](architecture/0027-windows-installer-lifecycle.md), [updates](architecture/0043-application-update-coordination.md) and [Store](architecture/0045-microsoft-store-distribution.md) decisions |
| How are diagnostic data bounded? | [Diagnostic export](architecture/0044-local-diagnostic-export.md) |
| Why was a technical boundary chosen? | [Architecture decisions](architecture/README.md) |
| Which checks apply, and which evidence can be reused? | [Acceptance index](acceptance/README.md), [quality gates](acceptance/quality-gates.md) and [UI verification map](acceptance/quality/ui-verification.md) |
| How should tests and documents be maintained? | [Testing](development/testing.md) and [documentation](development/documentation.md) standards |
| Where do modules belong? | [Repository layout](development/repository-layout.md) and [dependency boundaries](architecture/0037-feature-module-dependency-boundaries.md) |
| How do I change text or add a language? | [Internationalization](development/internationalization.md) |
| What changed in a published version? | [Release notes](releases/README.md) |
| What information does Ame process and retain? | [Privacy Statement](../PRIVACY.md) |
| How are security issues reported and evaluated? | [Security policy](../SECURITY.md) and [threat model](architecture/threat-model.md) |

Completed execution plans and one-time investigation logs remain in Git history. The active tree
retains current contracts, reusable procedures, pending plans and evidence that supports an active
qualification or technical choice.
