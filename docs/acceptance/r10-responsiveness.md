# R10a responsiveness evidence

The [roadmap](../roadmap.md#r10--beta-readiness) owns delivery status. This record supports
[R10-01](../plans/r10-beta-readiness.md#r10-01--responsiveness-and-release-baseline).

## Retained-library baseline — 2026-11-14

Baseline: `f1dcd2e27098b9342c8e462ff81ab752331d355d`, Windows x64, retained `local-primary`
catalog with 63,170 locations and 47,926 merged entries. Native measurements use the existing
optimized Profile-bundle DLL through the production Dart/Rust read API. Sources remain unchanged.
These timings locate backend costs; they are not input-to-rendered-content acceptance.

| Read | First process/read (ms) | Subsequent reads (ms) |
| --- | ---: | ---: |
| Merged gallery snapshot | 5,777 | 1,080 / 1,061 |
| Expanded gallery snapshot | 324 | 284 / 275 |
| Smart-album navigation | 1,763 | 901 / 920 |
| Three smart-category snapshots | 1,572 / 1,410 / 1,458 | 1,302–1,444 |
| Pending organization queue | 1,455 | 1,477 / 1,494 |
| Exact groups | 1,585 | 811 / 789 |
| Similar groups | 456 | 413 / 451 |
| Ignored exact / similar groups | 6 / 21 | 6–18 |

The first cold read in the session took 17,237 ms; later fresh-process reads were faster.
Temporary native instrumentation attributed the long startup to initial catalog validation and
the merged layout projection. In the instrumented run, projection cost 1,028 ms, its detail window
another 11 ms, and preview-state reconciliation 82 ms. Instrumentation was removed after diagnosis.

## Scoped similarity relationship validation

Similarity validation previously ran the global `pragma_foreign_key_check` and filtered its output
by table name. Filtering the output still traversed unrelated tables. It now selects similarity
tables first and passes each name into SQLite's table-specific check. The complete catalog check
remains in the catalog's existing validation path, and similarity relationship rejection is unchanged.

The focused upgrade, rollback and orphan-reference cases passed. The new case checks that an
unrelated orphan remains outside similarity ownership while an orphan similarity variant is rejected.
Local fixtures ran under the managed acceptance scratch directory after the host's default Temp
directory denied fixture creation. No fixture fallback or assertion was changed.
