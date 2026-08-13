# R2c-A blue-team validation

- Date: 2026-08-13
- Scope: platform-independent change planning and final-state reconciliation contracts
- Source access: none

## Threat model

The R2c-A planner treats every observation and reconciliation input as untrusted evidence. The
blue-team pass attacks ordering ambiguity, malformed paths and identities, incomplete rename
signals, stale root generations, unavailable roots, capacity exhaustion, supersession boundaries,
and evidence-accounting drift. An attack succeeds if equivalent batches produce different durable
plans, an incomplete signal can claim synchronized state, an invalid value can enter persistence,
or bounded input can create unbounded planner work.

This pass does not validate the future Windows watcher, durable queue, atomic delta publisher, or
real-library catch-up path. Those remain R2c-B through R2c-E gates.

## Findings and remediation

| Attack | Red-test result | Remediation |
| --- | --- | --- |
| Permute equal-sequence failures and mixed-origin batches | Issue order, fallback metadata, and origin could depend on arrival order | Use total deterministic observation and issue ordering; select the latest origin by sequence and observation time |
| Place malformed input inside or outside the bounded prefix | Overflow plans could differ according to which signal arrived first | Discard partial semantics after a proven overflow and emit one stable root freshness-unknown plan |
| Supply zero, oversized, or machine-maximum planning limits | Caller-controlled limits could disable the intended absolute bound | Enforce nonzero absolute ceilings of 4,096 observations and 1,024 intents |
| Insert NULs or empty identity components | Invalid persistence and identity evidence was accepted | Reject NUL paths and roots and preserve trustworthy state for invalid identity evidence |
| Claim authoritative absence for a different path | A missing signal was not bound to the inspected location | Carry and normalize the missing path; remove only when it matches prior evidence and absence is authoritative |
| Send a rename without an old path | Only the new path was reconciled while an unknown old location could remain live | Mark the batch as an evidence gap and require root reconciliation |
| Rename a normalized path to itself | A no-op alias was emitted as a rename candidate | Degrade to one path reconciliation |
| Exceed the intent limit just before a parent-directory signal | The planner degraded before valid subtree compaction | Compact path and nested-subtree work before applying the intent ceiling |
| Fold child work into a parent subtree | Child time ranges and event counts were discarded or one unpaired rename could be double-counted | Track bounded observation IDs internally and merge evidence without duplicating rename expansions |
| Present unknown, missing, inaccessible, or offline roots with no events | Freshness could be confused with availability | Keep every unavailable state explicit and never claim synchronized state |

## Regression evidence

Twenty adversarial Rust fixtures now cover:

- complete permutation invariance for representative mixed batches and overflow prefixes;
- malformed paths, root identifiers, and file identities;
- identity-scheme mismatch, path-bound authoritative absence, and unavailable roots;
- self-renames, incomplete renames, path and nested-subtree supersession;
- capacity ceilings, fallback evidence ranges, origin selection, and coalesced evidence counts.

Together with the 21 original directory-synchronization fixtures, all 41 focused application tests
pass. The complete repository Daily gate also passes: 173 Rust tests pass with five authorization-
or performance-bound tests intentionally ignored; all Flutter tests, controlled Windows scan
integration, native Windows accessibility integration, generated bridge compatibility, and
tracked-diff whitespace validation pass.

## Residual boundaries

- R2c-B must prove raw Windows events translate into these contracts without blocking, hydration,
  or dependency types crossing the adapter.
- R2c-C must preserve deterministic ordering, generation isolation, evidence counts, and capacity
  degradation through durable storage, leasing, retry, and crash recovery.
- R2c-D and R2c-E must prove atomic catalog publication, catch-up recovery, real-root read-only
  safety, and measured event-storm behavior.
