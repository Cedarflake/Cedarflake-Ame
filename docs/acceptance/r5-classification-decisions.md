# R5 classification decision verification

The [roadmap](../roadmap.md#r5-implementation-sequence) owns stage disposition. This record covers
the classification decision boundary under
[ADR 0035](../architecture/0035-classification-decision-authority.md). R5-03 still requires its
database, recovery and continuation integration before its exit conditions can be assessed.

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

The decision policy has no album, Favorite or organization-state mutation fields. Existing SQLite
and desktop workflow owners remain unchanged by this boundary implementation. This preserves the
integration seam without adding classification branches to R4's controllers or history owner.
