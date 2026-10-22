# First-import performance

Status: active; the measured candidate exceeds the one-minute acceptance limit

## Outcome and boundary

Reduce the time needed to finish the first import of a real image collection on a constrained
Windows computer. Preserve source read-only access, placeholder exclusion, bounded memory,
pause/cancel/resume, metadata interpretation, and validation before atomic catalog publication.
The complete cloud-primary library must finish first import within 60 seconds in the actual
Windows client, through validated atomic publication. This maintenance does not reopen R2c/R3 or
start R4.

## Execution

Measure the complete production-client import and the scan's existing operation-cost boundaries
on the affected computer. Separate source discovery, prior selection, media inspection, staging,
checkpoint persistence, and final publication before selecting the implementation owner.
Use isolated derived storage for comparisons and retain the client's existing catalog.

Repair the measured bottleneck at its owner, add the boundary regression needed to preserve its
correctness, and repeat the affected full-library client import. Synthetic or local-primary
measurements support diagnosis; neither predicts the constrained computer's result. Preserve the
original baseline and record build configuration and competing activity with the comparison.

The current gallery exposes a new root only after validated publication. Any change to this
publication behavior requires its own explicit application and presentation design; it is not
implied by reducing scan cost.

## Evidence

The production candidate amortizes automatic SQLite checkpoints only during a foreground import
into an empty catalog and reuses compiled prior-query statements. The continued candidate borrows
a 32 MiB page cache for the importing connection, uses a 32,768-page writeback trigger, and reuses
asset-insert and generation statements while preserving the 128-entry durable progress checkpoint.
The complete cloud-primary client
comparison accepted the same 19,028 locations in 145.448 seconds, including pause/resume, compared
with the original 794.052-second observation. Full member/metadata comparison and affected gallery
interactions passed. This candidate exceeds the subsequently selected 60-second limit. Continue
measuring and repairing the remaining import costs; keep PR #44 in draft until the requested
performance and applicable verification are satisfied. The subsequent production candidate finishes
in 80.381 seconds with the same 19,028 locations and zero bidirectional member/metadata differences.
The bounded source-inspection reader candidate took 80.605 seconds in the same client and did not
improve this result. Its extra scheduling machinery is removed; serial source inspection and the
measured catalog improvements remain. Continue with the remaining persistence and validation costs.

[First-import measurements](../acceptance/first-import-performance.md) owns timings, source safety,
diagnostic limitations, and verification. The roadmap owns the active maintenance queue.
