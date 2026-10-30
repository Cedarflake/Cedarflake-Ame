# R5 classification decision verification

The [roadmap](../roadmap.md#r5-implementation-sequence) owns stage disposition. This record covers
the classification decision boundary under
[ADR 0035](../architecture/0035-classification-decision-authority.md). The persistence, recovery and
logical-continuation implementation is connected; its new database regressions await hosted execution.
R5-05 owns eligible-group projection and R5-06 connects these internal commands to desktop review.

## Evidence and command policy — 2026-10-30

The existing matcher now emits validated domain rankings. Screening results translate directly to
immutable run/content evidence, retaining the primary ranking, optional refined ranking and exact
profile identity. A skipped refinement still binds its admitted configuration. No model, file or
database operation is introduced by this translation.

The application decision owner checks observed/current content, membership and classification-field
authority. Confirmation binds the displayed prediction; correction uses an explicit active category.
Human consensus preserves local/imported origin, compatible choices and conflicts. Effective state
keeps human choices through model absence and prevents machine suggestions from becoming confirmed.

Focused checks use 17 hash-matched production/test source files in the existing ignored diagnostic
harness with the repository's pinned `blake3` and `thiserror` versions. Local execution remained
serial under the repository lock. Results:

- 74 tests passed, none failed or ignored; 35 cover the new evidence/decision boundary and 39 retain
  the category, matching and screening checks.
- Focused Clippy passed with warnings denied.
- Matching on the retained 192 native image features agrees with the frozen reference for category,
  review recommendation and refinement in all 192 cases. This consumes cached features and performs
  no original-media reads or inference.
- Owned Rust formatting and `git diff --check` pass.

The rejecting cases include invalid/duplicate rankings, mismatched models/rules/categories, stale
run/profile evidence, changed content or membership, intervening category/review decisions, retired
destinations and old confirmations after a replacement prediction with the same category. Tests also
cover model-independent correction/deferral, same-category imported choices becoming explicit local
intent, conflicting human choices, provisional correction and unavailable prediction evidence.

The initial focused compilation found that an older matching test obtained `CategoryId` through its
parent's incidental import. That test now imports its domain dependency directly. The corrected
source passed the checks above. No requirement or warning gate was weakened.

These checks exercise domain/application policy and its real screening translation. They do not
establish database transactions, restart/rebuild, undo, source removal, archive restoration,
continuation or desktop interaction; those require their persistence and connected workflow checks.
The complete crate and hosted workloads remain owned by exact-commit CI.

## Physical ownership

Counts include blank lines and module declarations; dedicated tests remain outside production files.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Immutable run/prediction evidence | 223 | 0 | 252 |
| Validated ranking | 73 | 0 | Included in evidence tests |
| Human consensus and effective state | 228 | 0 | Included in decision tests |
| Guarded category/review decision policy | 107 | 0 | 514 |
| Screening and evidence translation | 170 | 0 | 97 new; existing screening tests retained |
| Category matching | 90 | 0 | Existing matcher tests retained |

The decision policy has no album, Favorite or organization-state mutation fields. The initial
boundary implementation preserved existing SQLite and desktop owners; the integration below adds
separate classification storage and narrow source/recovery hooks.

## Persistence and recovery integration — 2026-10-30

Schema 65 adds immutable category definitions, runs and prediction evidence, independent human
category/review fields, guarded operation history and a classification-only continuation record.
Commands revalidate the complete selected batch before writing and retain opaque undo targets after
source removal or explicit cleanup. Group reconciliation reuses the existing R4 lineage identity.
No original-media reads, writes, placeholder hydration or new model downloads are introduced.

Archive version 5 retains versions 1–4 and includes category definitions, human choices, operation
history and logical continuation. Restoration shares the existing atomic transaction, remaps source,
location and admission identities, and preserves current local choices. Prediction caches, active
worker configuration and live review sessions remain outside portable recovery. Cleanup inspection
includes classification records in its authority digest and displays their affected group count in
the existing confirmation dialog.

Classification continuation has its own session and position sequence. A category action and its
resulting continuation position commit together. Superseded sessions, old checkpoints and stale
command targets cannot replace newer state. Ordinary classification retains the current image;
deferral or a result outside the current filter advances its logical boundary. R4 organization
sessions, bookmarks, album decisions and Favorites retain independent authority.

Local checks on the integrated source:

- `cargo check --tests --locked --offline -j 1` passed, including the new archive, migration,
  source-lifecycle, independent-session and cleanup regressions.
- Canonical bridge generation completed and its native Release build passed in 5 minutes 1 second.
- The focused cleanup-confirmation widget suite passed all 5 tests, including affected classification
  counts, cancellation, changed inspection ranges, late completion and retry.
- Owned Rust/Dart formatting and `git diff --check` passed.

An earlier focused database execution passed 85 tests and failed one late-group fixture that had
omitted exact-batch publication. The fixture now publishes that batch before observing the group.
The subsequent expanded test compilation stopped in LLVM with an out-of-memory error, so it supplied
no runtime result for the newer database scenarios. The final compile check passed; runtime migration,
archive/restart, source-removal/reimport, rebuild, undo and continuation results remain assigned to
the exact-commit hosted checks. No test workload or assertion was removed.

The previous commit `5556aa9c` passed all applicable jobs in
[run 41055517195](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41055517195).
That result covers the earlier decision boundary, not this persistence integration. Classification
desktop interaction and next-eligible navigation remain in their planned connected slices.

### Integration ownership

Production modules keep transaction publication, observation authority, field undo, review position,
source lifecycle and archive restoration separate. Dedicated tests are excluded from production counts.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| Classification validation | 229 | 0 | Shared authority and lifecycle suites |
| Storage codec / prediction codec | 219 / 127 | 0 | Shared classification suites |
| Configuration / observation / publication | 162 / 165 / 189 | 0 | 225 baseline + 177 authority |
| Command / request / undo / operation publication | 100 / 103 / 134 / 122 | 0 | 283 lifecycle |
| Review facade / scope / position / source lifecycle / command | 95 / 166 / 135 / 135 / 102 | 0 | 269 review |
| Migration schema | 122 | 0 | 45 migration |
| Archive validation / restore history / bookmark association | 200 / 41 / 87 | 0 | 201 archive |
