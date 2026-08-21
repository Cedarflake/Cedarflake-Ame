# R2c-J metadata-inventory persistence and discovery acceptance

Status: audit remediation implemented; final independent re-audit pending

Date: 2026-08-21

## Scope

R2c-J implements the persistence and discovery boundary defined by ADR 0023. It does not schedule
production startup inventory or continuity epochs; those orchestration responsibilities remain in
R2c-K.

## Implemented contract

- Schema v20 adds exact-shape inventory run, staging, cursor, completion-authority, cleanup, and
  comparison indexes outside source roots.
- Terminal staging cleanup is limited to 4,096 entries per transaction; terminal run summaries are
  retained for seven days and removed in batches of at most 128.
- The v19-to-v20 migration preserves change identifiers, queue state, retries, leases,
  authoritative scan ownership, self-references, catch-up evidence, and queue lineage while adding
  the `metadata_inventory` origin.
- Current-schema validation checks exact inventory columns and indexes, the active-root index
  predicate, cascade relationships, the queue origin contract, marker completion, root generation,
  published-catalog ownership, staged counts, scope containment, and foreign-key integrity.
- Local inventory walks root or subtree scopes in pages of at most 4,096 entries and records path,
  entry kind, size, modification time, optional Windows identity, reparse evidence, and placeholder
  state.
- Inventory does not inspect media signatures, decode images, generate previews, hash or read source
  bytes, follow intermediate or terminal reparse directories, or open identity handles for offline
  or recall placeholders.
- Terminal directory symlinks and junctions, and reparse targets that cannot be classified safely,
  fail the inventory without authorizing descendant absence.
- Positive differences enqueue the existing final-state path reconciler. Same-root identity evidence
  may create one paired rename candidate without reusing a previous path for another hard link.
- Missing paths are loaded only after complete enumeration establishes scope-wide absence authority.
  Missing subtrees produce an empty complete page; partial, failed, cancelled, stale-generation, or
  malformed runs never authorize absence.
- Any failure after durable run creation terminates the run with a bounded structured issue. A
  cancelled run remains non-authoritative unless enumeration had already completed; already
  enqueued final-state work remains safe and idempotent.
- Terminalization retries bounded catalog contention and reports a combined error if both the
  inventory operation and terminalization fail. A newer generation or epoch atomically supersedes
  an older active run, after which bounded cleanup releases its staging.
- Unknown-extension final-state reconciliation distinguishes unsupported signatures from unreadable
  files. An unreadable signature retries without removing the last trustworthy catalog location.

## Focused verification

- metadata inventory: 14 passed, covering fixed-bound all-entry enumeration, empty missing subtree,
  controlled closed-process create/modify/delete/rename/directory move, Chinese and long paths,
  placeholder evidence, cancellation, durable failure, hard-link pairing, reparse-directory
  rejection, bounded cleanup, orphan supersession, and terminalization retry;
- locked wrong-extension reconciliation: passed, proving unreadable signature evidence retains the
  last trustworthy catalog location and durable retry work;
- schema migrations: 29 passed, including v19 queue/lineage preservation, new-origin admission,
  exact-index and predicate rejection, missing-origin rejection, stale-generation rejection, and all
  retained v17-v19 compatibility fixtures;
- offline placeholder adapter: passed, proving inventory records the placeholder without file
  identity and preserves source bytes;
- complete Rust suite: 438 total, 431 passed, seven existing explicit ignores, zero failures;
- `./tool/quality_lint.ps1`: passed, including repository guardrails, formatting, Clippy with
  warnings denied, and Dart analysis;
- `./tool/quality_verify_daily.ps1`: passed, including the complete Rust and Flutter suites,
  Windows scan integration, Windows accessibility integration, bridge compatibility, and
  whitespace validation;
- `./tool/release_verify_windows.ps1`: passed, including the Windows Release build and two packaged
  bridge smoke tests;
- independent read-only audit: pending.

The focused fixtures use disposable directories and isolated catalogs. They do not access
`local-primary` or `cloud-primary`, hydrate cloud content, or modify any real source media.

## Next boundary

R2c-K owns production continuity epochs, cold-start and watcher-gap scheduling, durable page
continuation, live-event supersession, bounded backpressure, restart semantics, and fairness. R2c-J
does not claim those production lifecycle behaviors.
