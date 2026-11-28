# Verification and evidence

The [roadmap](../roadmap.md) owns delivery status. This directory contains reusable verification
procedures and evidence needed for the current Beta, distribution paths and admitted engines.
Measurements identify their revision, environment and limitations; reuse them within that scope.

## Procedures

| Task | Owner |
| --- | --- |
| Select and run checks | [Quality gates](quality-gates.md) |
| Verify preview and gallery geometry | [Gallery regression contract](gallery-foundation.md) |
| Verify synchronization, publication and recovery | [Library continuity contract](library-continuity.md) |
| Read real source libraries safely | [Read-only library procedure](read-only-real-library.md) |
| Verify copy, move, deletion and recovery | [Filesystem procedure](r8-file-operations.md) |

## Candidate and distribution evidence

| Record | Purpose |
| --- | --- |
| [R10 responsiveness](r10-responsiveness.md) | Retained-library startup, navigation and rendering baseline |
| [R10 resources and recovery](r10-resource-recovery.md) | Sustained use, cache/index reuse and interruption recovery |
| [R10 diagnostics](r10-diagnostics.md) | Local export, privacy projection and startup-failure access |
| [Supported prerelease baseline](prerelease-compatibility.md) | Current database integrity and retirement of obsolete Alpha adaptations |
| [Windows distribution](unsigned-windows-release.md) | Artifact admission, reuse and multilingual MSI maintenance |
| [Application updates](application-updates.md) | Verified packages, joined restart and installed/portable recovery |
| [Microsoft Store](store-msix.md) | Package identity, data lifecycle and remaining Store verification |

## Engine selection and calibration

| Record | Purpose |
| --- | --- |
| [Model comparison](r5-model-evaluation.md) | Encoder selection and provisional-label limitations |
| [Screening calibration](r5-calibration.md) | Separated calibration/holdout method and threshold provenance |
| [Inference runtime](r5-runtime-admission.md) | Pinned artifacts, input compatibility and isolated-worker measurements |
| [GPU analysis](gpu-analysis-acceleration.md) | Precision agreement, resource cost and device-switching evidence |
| [Similarity engine](r6-engine-evaluation.md) | Hash parameters, sampled quality, bounded indexing and replacement tradeoffs |

Historical stage reports are available in [Git history](https://github.com/Cedarflake/Cedarflake-Ame/tree/e5874f1bed869075019a6626768d7aaf2c227dcb/docs/acceptance).
Keep raw local logs, private catalogs, source media and machine-specific paths outside tracked documents.
