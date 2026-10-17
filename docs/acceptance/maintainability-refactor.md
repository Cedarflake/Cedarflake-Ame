# Maintainability refactor evidence

Date: 2026-10-17

Scope and execution order belong to the [active plan](../plans/maintainability-refactor.md).
This record covers changed boundaries, not another R2c/R3 stage acceptance run.

## Shared schema-shape validation

The extraction preserves function bodies, query order, version-dependent scan-handoff columns,
case-insensitive declared types, exact ordered column/index comparisons, the single cascade foreign
key requirement and quoted SQL tokens. Contract/repair callers import the shared owner directly.
No schema version, migration order, dependency or source-read policy changes.

| Owner | Production/support lines | Inline test-only lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `migrations.rs` before | 9,762 | 5,358 | 0 |
| `migrations.rs` after | 9,581 | 5,327 | 0 |
| `migrations/schema_shape.rs` | 193 | 2 | 165 |

Counts include whitespace. Test-only counts include `cfg(test)` hooks/declarations as well as the
inline test module; the two lines in `schema_shape.rs` declare its dedicated test module. Existing
tests of other migration owners are outside this table. One canonicalization regression moved and
five boundary regressions were added; migration integration tests remain in place.

Local verification:

- Scoped Cargo library filter `schema_shape::tests`, serialized through the repository tool lock:
  6 passed, 0 failed, 0 ignored. The clean run used `CARGO_INCREMENTAL=0` after the sandbox's
  incremental-cache hard-link attempt emitted a fallback warning; no lint or test rule was disabled.
- `quality_format.ps1 -Check`: passed, including all 410 Dart files unchanged. The sandbox attempt
  formatted successfully but could not write Dart's analytics session; the identical check passed
  with normal tool permissions.
- Final source/diff review and `git diff --check`: passed. Historical migrations, repair admission
  and the source catalog remain unchanged by this extraction.
- Hosted checks: pending first branch delivery. No hosted or client pass is claimed by this record.

Remaining M1 work is historical contract/row validation and exact compatibility-repair ownership.
